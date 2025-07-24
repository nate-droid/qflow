// In your vqa-runner crate...

// Make sure you have your simulator and its gate types in scope
use qsim::{QuantumSimulator, Gate}; // Or however you import them

/// Applies a hardware-efficient ansatz to the simulator.
///
/// This specific ansatz uses layers of Y-rotations and CNOTs.
/// The number of parameters must match the requirements of the circuit.
/// For this example, it needs 2 * num_layers parameters.
pub fn apply_ansatz(simulator: &mut QuantumSimulator, params: &[f64]) {
    let num_qubits = simulator.num_qubits();
    let num_layers = 2; // A hyperparameter you can tune

    // Ensure we have the correct number of parameters.
    // Each layer has a rotation on each qubit.
    assert_eq!(
        params.len(),
        num_qubits * num_layers,
        "Incorrect number of parameters for the ansatz"
    );

    let mut params_iter = params.iter();

    for layer in 0..num_layers {
        // 1. Layer of single-qubit rotation gates
        for i in 0..num_qubits {
            let theta = params_iter.next().unwrap();
            // You'll need to have implemented a parameterized Ry gate
            // in your simulator.
            simulator.apply_gate(&Gate::RY(i, *theta));
        }

        // 2. Layer of entangling gates
        // Here, we entangle each qubit with its neighbor.
        for i in 0..(num_qubits - 1) {
            simulator.apply_gate(&Gate::CX(i, i + 1));
        }
    }
}