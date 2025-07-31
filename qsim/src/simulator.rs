use super::parser::{Gate, parse_qasm};
use super::state::StateVector;
use crate::circuit::Circuit;
use crate::events::{Event, GateInfo, MeasurementInfo, SimulationStartInfo};
use num_complex::Complex;
use std::f64::consts::FRAC_1_SQRT_2;

pub trait Simulator {
    /// Resets the simulator to the |0...0⟩ state.
    fn reset(&mut self);
    /// Applies a single quantum gate to the state.
    fn apply_gate(&mut self, gate: &Gate);
    /// Measures the expectation value of a given Pauli string.
    /// The internal state |ψ⟩ is not changed. The measurement is performed
    /// by applying the Pauli operators P to a copy of the state and
    /// calculating ⟨ψ|P|ψ⟩.
    fn measure_pauli_string_expectation(&mut self, operators: Vec<Gate>) -> f64;

    fn get_statevector(&self) -> &StateVector;
    fn get_num_qubits(&self) -> usize;

    // compile the circuit to openqasm
    fn compile_to_qasm(&self) -> String;
}

pub trait QuantumGate {
    fn apply(&self, state: &mut [Complex<f64>]);
}

pub struct QuantumSimulator {
    pub num_qubits: usize,
    pub state: StateVector,
}

impl Simulator for QuantumSimulator {
    fn reset(&mut self) {
        self.state.reset();
    }
    fn apply_gate(&mut self, gate: &Gate) {
        match gate {
            Gate::H{qubit} => self.state.apply_single_qubit_gate(&HADAMARD, *qubit),
            Gate::X{qubit} => self.state.apply_single_qubit_gate(&PAULI_X, *qubit),
            Gate::Y{qubit} => self.state.apply_single_qubit_gate(&PAULI_Y, *qubit),
            Gate::Z{qubit} => self.state.apply_single_qubit_gate(&PAULI_Z, *qubit),
            Gate::CX {control, target} | Gate::CNOT {control, target} => self.state.apply_cx(*control, *target),
            Gate::Measure => {
                let result = self.state.measure_all(&mut rand::thread_rng());
            }
            _ => {
                let matrix = construct_gate_matrix(gate);

                if let Some(matrix) = matrix {
                    if gate.target().len() == 1 {
                        self.state
                            .apply_single_qubit_gate(&matrix, gate.target()[0]);
                    } else {
                        self.state.apply_multi_qubit_gate(&matrix, &gate.target());
                    }
                } else {
                    eprintln!("Unsupported gate type: {:?}", gate);
                    panic!("Unsupported gate type encountered during simulation.");
                }
            }
        }
    }

    fn measure_pauli_string_expectation(&mut self, operators: Vec<Gate>) -> f64 {
        use num_complex::Complex;

        // Save the original state
        let original_state = self.state.amplitudes.clone();

        // Apply the Pauli string operator to the state
        for op in &operators {
            match op {
                Gate::X{qubit} => self.state.apply_single_qubit_gate(&PAULI_X, *qubit),
                Gate::Y{qubit} => self.state.apply_single_qubit_gate(&PAULI_Y, *qubit),
                Gate::Z{qubit} => self.state.apply_single_qubit_gate(&PAULI_Z, *qubit),
                _ => panic!("Unsupported operator in Pauli string expectation"),
            }
        }

        // Compute the inner product <psi|P|psi>
        let mut expectation = Complex::new(0.0, 0.0);
        for (a, b) in original_state.iter().zip(self.state.amplitudes.iter()) {
            expectation += a.conj() * b;
        }

        // Restore the original state
        self.state.amplitudes = original_state;

        expectation.re
    }

    fn get_statevector(&self) -> &StateVector {
        &self.state
    }

    fn get_num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn compile_to_qasm(&self) -> String {
        todo!("Implement QASM compilation for the simulator");
        // This method would typically convert the current state of the simulator
        // into a QASM representation. For simplicity, we return an empty string here.
        String::new()
    }
}

impl QuantumSimulator {
    pub fn new(num_qubits: usize) -> Self {
        QuantumSimulator {
            num_qubits,
            state: StateVector::new(num_qubits),
        }
    }

    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    pub fn apply_circuit(&mut self, circuit: &Circuit) {
        for moment in &circuit.moments {
            for gate in moment {
                self.apply_gate(gate);
            }
        }
    }

    // sets the simulator state to a specific configuration ie: [0, 0, 1, 0, 0] == "00100"
    pub fn prepare_initial_state(&mut self, initial_state: &[u8]) {
        for (i, &state) in initial_state.iter().enumerate() {
            if state == 1 {
                // Apply an X gate to flip |0> to |1>
                self.apply_gate(&Gate::X { qubit: i });
            }
        }
    }

    pub fn get_probability(&self, state_index: usize) -> f64 {
        if state_index >= self.state.amplitudes.len() {
            eprintln!("Error: State index out of bounds.");
            return 0.0;
        }
        let amp = self.state.amplitudes[state_index];
        (amp.re * amp.re + amp.im * amp.im).sqrt()
    }

    fn parse_pauli_term(&self, term_str: &str) -> Result<Vec<Gate>, String> {
        term_str.split_whitespace().map(|pauli_op| {
            let op_char = pauli_op.chars().next()
                .ok_or_else(|| "Empty Pauli operator in string".to_string())?;
            let qubit_idx = pauli_op[1..].parse::<usize>()
                .map_err(|_| format!("Invalid qubit index in '{}'", pauli_op))?;

            if qubit_idx >= self.num_qubits as usize {
                return Err(format!("Qubit index {} is out of bounds for {} qubits.", qubit_idx, self.num_qubits));
            }

            match op_char {
                'X' => Ok(Gate::X{qubit: qubit_idx}),
                'Y' => Ok(Gate::Y{qubit: qubit_idx}),
                'Z' => Ok(Gate::Z{qubit: qubit_idx}),
                'I' => Ok(Gate::I{qubit: qubit_idx}),
                _ => Err(format!("Unknown Pauli operator '{}'", op_char)),
            }
        }).collect()
    }

    pub fn measure_expectation(&self, operator_string: &str, shots: usize) -> Result<f64, String> {
        // For simplicity, this example only handles single-term operators like "Z0 X1".
        // A full implementation would need to handle coefficients and multiple terms
        // like "1.5 * Z0 - 0.5 * X1".

        let pauli_terms = self.parse_pauli_term(operator_string)?;

        let mut total_eigenvalue = 0.0;

        for _ in 0..shots {
            // In a real simulator, you would sample from the final state vector's probabilities.
            // For this example, we'll simulate a simple case to demonstrate the logic.
            // Let's assume the measurement always results in the |0...0> state.
            let measurement_outcome = 0; // Represents the integer value of the bitstring, e.g., "01" -> 1

            let mut shot_eigenvalue = 1.0;
            for (pauli) in &pauli_terms {
                // Get the bit value for the specific qubit from the measurement outcome.
                let bit = (measurement_outcome >> pauli.target()[0]) & 1;

                // Determine the eigenvalue (+1 or -1) for this Pauli measurement.
                // For Z, |0> is +1, |1> is -1.
                // For X and Y, the eigenvalue depends on the superposition, but for the
                // basis states, we can define a consistent (though simplified) mapping.
                let eigenvalue = match pauli {
                    Gate::Z{..} => if bit == 0 { 1.0 } else { -1.0 },
                    // For a real simulation, X and Y measurements require basis changes before measuring.
                    // Here we provide a placeholder result.
                    Gate::X{..} => 1.0,
                    Gate::Y{..} => 1.0,
                    Gate::I{..} => 1.0,
                    _ => return Err(format!("Unsupported Pauli operator: {:?}", pauli)),
                };
                shot_eigenvalue *= eigenvalue;
            }
            total_eigenvalue += shot_eigenvalue;
        }

        // The expectation value is the average of all the single-shot eigenvalues.
        Ok(total_eigenvalue / shots as f64)
    }
}

// custom type for gate matrices
pub type GateMatrix = [[Complex<f64>; 2]; 2];

pub const HADAMARD: GateMatrix = [
    [
        Complex::new(FRAC_1_SQRT_2, 0.0),
        Complex::new(FRAC_1_SQRT_2, 0.0),
    ],
    [
        Complex::new(FRAC_1_SQRT_2, 0.0),
        Complex::new(-FRAC_1_SQRT_2, 0.0),
    ],
];

pub const PAULI_X: GateMatrix = [
    [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
    [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
];

pub const PAULI_Y: GateMatrix = [
    [Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
    [Complex::new(0.0, 1.0), Complex::new(0.0, 0.0)],
];

pub const PAULI_Z: GateMatrix = [
    [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
    [Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
];

pub fn construct_gate_matrix(gate: &Gate) -> Option<GateMatrix> {
    match gate {
        Gate::RX{qubit, theta} => Some([
            [
                Complex::new((theta / 2.0).cos(), 0.0),
                Complex::new(0.0, -(theta / 2.0).sin()),
            ],
            [
                Complex::new(0.0, -(theta / 2.0).sin()),
                Complex::new((theta / 2.0).cos(), 0.0),
            ],
        ]),
        Gate::RY{qubit, theta} => Some([
            [
                Complex::new((theta / 2.0).cos(), 0.0),
                Complex::new(0.0, -(theta / 2.0).sin()),
            ],
            [
                Complex::new(0.0, (theta / 2.0).sin()),
                Complex::new((theta / 2.0).cos(), 0.0),
            ],
        ]),
        Gate::RZ{qubit, theta} => Some([
            [
                Complex::new((theta / 2.0).cos(), -(theta / 2.0).sin()),
                Complex::new(0.0, 0.0),
            ],
            [
                Complex::new(0.0, 0.0),
                Complex::new((theta / 2.0).cos(), (theta / 2.0).sin()),
            ],
        ]),
        _ => {
            eprintln!("Unsupported gate type: {:?}", gate);
            panic!("Unsupported gate type encountered during simulation.");
        } // Unsupported gate type
    }
}

pub fn run_simulation(qasm_input: &str) -> Option<Vec<Event>> {
    let mut events = Vec::new();

    let (num_qubits, gates) = parse_qasm(qasm_input);
    if num_qubits == 0 {
        eprintln!("Error: Could not determine number of qubits from QASM input.");
        return None;
    }

    events.push(Event::SimulationStart(SimulationStartInfo {
        num_qubits,
        num_gates: gates.len(),
    }));

    let mut state = StateVector::new(num_qubits);
    let mut rng = rand::thread_rng();

    for (i, gate) in gates.iter().enumerate() {
        let gate_str = format!("{:?}", gate);
        match gate {
            Gate::H{qubit} => state.apply_single_qubit_gate(&HADAMARD, *qubit),
            Gate::X{qubit} => state.apply_single_qubit_gate(&PAULI_X, *qubit),
            Gate::Y{qubit} => state.apply_single_qubit_gate(&PAULI_Y, *qubit),
            Gate::Z{qubit} => state.apply_single_qubit_gate(&PAULI_Z, *qubit),
            Gate::CX{control, target} | Gate::CNOT {control, target} => state.apply_cx(*control, *target),
            Gate::Measure => {
                let result = state.measure_all(&mut rng);

                events.push(Event::MeasurementResult(MeasurementInfo {
                    classical_outcome: result,
                    binary_outcome: format!("{:b}", result),
                    final_state_vector: state.clone(),
                }));
                return Some(events); // Simulation ends on measurement.
            }
            _ => {
                eprintln!("Unsupported gate: {:?}", gate);
                panic!("Unsupported gate type encountered during simulation.");
            }
        }

        events.push(Event::GateApplication(GateInfo {
            step: i + 1,
            gate: gate_str,
            state_vector: state.clone(),
        }));
    }
    Some(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPSILON: f64 = 1e-9;

    fn approx_eq(a: Complex<f64>, b: Complex<f64>) -> bool {
        (a.re - b.re).abs() < EPSILON && (a.im - b.im).abs() < EPSILON
    }

    #[test]
    fn test_bell_state_simulation() {
        let mut state = StateVector::new(2);
        state.apply_single_qubit_gate(&HADAMARD, 0);
        state.apply_cx(0, 1);
        let expected_amp = Complex::new(FRAC_1_SQRT_2, 0.0);
        assert!(approx_eq(state.amplitudes[0], expected_amp));
        assert!(approx_eq(state.amplitudes[1], Complex::new(0.0, 0.0)));
        assert!(approx_eq(state.amplitudes[2], Complex::new(0.0, 0.0)));
        assert!(approx_eq(state.amplitudes[3], expected_amp));
    }
}
