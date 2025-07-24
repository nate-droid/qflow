// in vqa-runner/src/main.rs

use hamiltonian::{Hamiltonian, PauliTerm};
use std::cell::RefCell;
use std::str::FromStr;
// The `Simulator` trait is now expected to be defined in your `qsim` crate
// and implemented for `StatevectorSimulator`. You should move the trait definition
// from this file into your `qsim` crate.
use qsim::{Gate, QuantumSimulator as StatevectorSimulator};
use qsim::simulator::Simulator;
// The `Simulator` trait definition has been REMOVED from this file.
// It should be moved to your `qsim` crate.

/// A VQE problem runner that is configured with a specific Hamiltonian and ansatz circuit.
/// It is generic over any type `S` that implements the `Simulator` trait.
pub struct VqeRunner<S, F>
where
    S: Simulator,
    F: Fn(&mut S, &[f64]) + Copy,
{
    simulator: RefCell<S>,
    hamiltonian: Hamiltonian,
    ansatz: F,
}

impl<S, F> VqeRunner<S, F>
where
    S: Simulator,
    F: Fn(&mut S, &[f64]) + Copy,
{
    /// Creates a new VQE runner, configured with a simulator, a Hamiltonian,
    /// and the ansatz circuit to use.
    pub fn new(simulator: S, hamiltonian: Hamiltonian, ansatz: F) -> Self {
        VqeRunner {
            simulator: RefCell::new(simulator),
            hamiltonian,
            ansatz,
        }
    }

    /// Calculates the expectation value of the Hamiltonian for a given
    /// set of parameters. This is our cost function.
    pub fn cost_function(&self, params: &[f64]) -> f64 {
        let mut total_energy = 0.0;

        for pauli_term in &self.hamiltonian.terms {
            let mut simulator = self.simulator.borrow_mut();
            simulator.reset();
            (self.ansatz)(&mut simulator, params);

            // Convert the pauli term to a vector of Gates
            let gates: Vec<Gate> = pauli_term
                .operators
                .iter()
                .map(|(pauli, qubit)| match pauli {
                    hamiltonian::Pauli::I => Gate::I(*qubit),
                    hamiltonian::Pauli::X => Gate::X(*qubit),
                    hamiltonian::Pauli::Y => Gate::Y(*qubit),
                    hamiltonian::Pauli::Z => Gate::Z(*qubit),
                })
                .collect();

            // The expectation is calculated on the immutable state, as per the trait definition.
            let expectation = simulator.measure_pauli_string_expectation(gates);
            total_energy += pauli_term.coefficient * expectation;
        }
        total_energy
    }

    /// Calculates the gradient of the cost function with respect to all parameters
    /// using the parameter-shift rule.
    pub fn gradient(&self, params: &[f64]) -> Vec<f64> {
        let mut gradient = vec![0.0; params.len()];
        let mut temp_params = params.to_vec();
        let shift = std::f64::consts::FRAC_PI_2; // pi / 2

        for i in 0..params.len() {
            temp_params[i] += shift;
            let energy_plus = self.cost_function(&temp_params);

            temp_params[i] -= 2.0 * shift;
            let energy_minus = self.cost_function(&temp_params);

            temp_params[i] += shift;
            gradient[i] = 0.5 * (energy_plus - energy_minus);
        }
        gradient
    }

    /// Runs the VQE optimization using simple gradient descent.
    pub fn run(
        &self,
        initial_params: Vec<f64>,
        steps: usize,
        learning_rate: f64,
    ) -> (f64, Vec<f64>) {
        let mut params = initial_params;

        for _ in 0..steps {
            let grad = self.gradient(&params);
            for j in 0..params.len() {
                params[j] -= learning_rate * grad[j];
            }
        }
        let final_energy = self.cost_function(&params);
        (final_energy, params)
    }
}

// --- Main Application: H2 Molecule Dissociation Curve ---

/// A hardware-efficient ansatz for two qubits.
fn two_qubit_ansatz<S: Simulator>(simulator: &mut S, params: &[f64]) {
    simulator.apply_gate(&Gate::RY(0, params[0]));
    simulator.apply_gate(&Gate::RY(1, params[1]));
    simulator.apply_gate(&Gate::CX(0, 1));
    simulator.apply_gate(&Gate::RY(0, params[2]));
    simulator.apply_gate(&Gate::RY(1, params[3]));
}

/// Returns the H2 molecule Hamiltonian for a given internuclear distance (in Angstroms).
/// Coefficients are pre-computed from quantum chemistry calculations.
fn get_h2_hamiltonian_at_distance(distance: f64) -> Hamiltonian {
    // Coefficients obtained from various quantum chemistry tutorials.
    // A more robust implementation would calculate these from integrals.
    let (c_i, c_z0, c_z1, c_z0z1, c_x0x1) = match (distance * 100.0) as u32 {
        74 => (-0.8126, 0.1712, -0.2228, 0.1686, 0.0453), // Equilibrium
        90 => (-0.7386, 0.1656, -0.2139, 0.1659, 0.0453),
        120 => (-0.6120, 0.1507, -0.1915, 0.1568, 0.0453),
        150 => (-0.5028, 0.1343, -0.1688, 0.1468, 0.0453),
        180 => (-0.4226, 0.1203, -0.1504, 0.1384, 0.0453),
        210 => (-0.3642, 0.1088, -0.1356, 0.1317, 0.0453),
        _ => panic!("No pre-computed Hamiltonian for distance {}", distance),
    };

    Hamiltonian::new()
        .with_term(PauliTerm::new().with_coefficient(c_i)) // Identity term
        .with_term(PauliTerm::new().with_coefficient(c_z0).with_pauli(0, hamiltonian::Pauli::Z))
        .with_term(PauliTerm::new().with_coefficient(c_z1).with_pauli(1, hamiltonian::Pauli::Z))
        .with_term(PauliTerm::new().with_coefficient(c_z0z1).with_pauli(0, hamiltonian::Pauli::Z).with_pauli(1, hamiltonian::Pauli::Z))
        .with_term(PauliTerm::new().with_coefficient(c_x0x1).with_pauli(0, hamiltonian::Pauli::X).with_pauli(1, hamiltonian::Pauli::X))
}

fn main() {
    println!("--- Calculating H2 Molecule Dissociation Curve ---");

    let distances = vec![0.74, 0.9, 1.2, 1.5, 1.8, 2.1];
    let mut results = Vec::new();

    for &distance in &distances {
        println!("\n--- Running VQE for distance: {} Å ---", distance);
        let h2_hamiltonian = get_h2_hamiltonian_at_distance(distance);

        let simulator = StatevectorSimulator::new(2);
        let vqe_runner = VqeRunner::new(simulator, h2_hamiltonian, two_qubit_ansatz);

        let initial_params = vec![0.1, 0.2, 0.3, 0.4];
        let steps = 100;
        let learning_rate = 0.4;

        let (final_energy, _) = vqe_runner.run(initial_params, steps, learning_rate);
        results.push((distance, final_energy));
    }

    println!("\n\n--- H2 Dissociation Curve Results ---");
    println!("---------------------------------------");
    println!("| Distance (Å) | Ground State Energy |");
    println!("|--------------|---------------------|");
    for (distance, energy) in results {
        println!("| {:<12.2} | {:<19.8} |", distance, energy);
    }
    println!("---------------------------------------");
}

// --- Test Module ---

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple ansatz for a single qubit problem.
    fn single_qubit_ansatz<S: Simulator>(simulator: &mut S, params: &[f64]) {
        simulator.apply_gate(&Gate::RY(0, params[0]));
    }

    #[test]
    fn test_vqe_for_single_qubit_z() {
        let hamiltonian = Hamiltonian::new()
            .with_term(PauliTerm::new().with_coefficient(1.0).with_pauli(0, hamiltonian::Pauli::Z));

        let simulator = StatevectorSimulator::new(1);
        let vqe_runner = VqeRunner::new(simulator, hamiltonian, single_qubit_ansatz);

        let initial_params = vec![0.1];
        let steps = 100;
        let learning_rate = 0.4;

        let (final_energy, _final_params) =
            vqe_runner.run(initial_params, steps, learning_rate);

        let expected_energy = -1.0;
        assert!(
            (final_energy - expected_energy).abs() < 1e-6,
            "Final energy {} is not close to expected energy {}",
            final_energy,
            expected_energy
        );
    }
}
