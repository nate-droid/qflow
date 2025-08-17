pub mod parser;
pub mod simulator;
pub mod state;

pub mod circuit;
pub mod events;
pub mod api;
pub mod statevector_backend;
pub mod facade;

pub use parser::{Gate, parse_qasm};
pub use simulator::QuantumSimulator;
pub use simulator::run_simulation;
pub use state::StateVector;

#[cfg(test)]
mod tests {
    use super::{Gate, QuantumSimulator};
    use crate::circuit::Circuit;
    use std::f64::consts::PI;

    const EPSILON: f64 = 1e-10;
    #[test]
    fn test_ry_rotation_to_one() {
        let mut simulator = QuantumSimulator::new(1);
        let mut circuit = Circuit::new();

        circuit.add_gate(Gate::RY {
            qubit: 0,
            theta: PI,
        });

        simulator.apply_circuit(&circuit);

        // State |1> is at index 1
        let prob_1 = simulator.get_probability(1);

        assert!((prob_1 - 1.0).abs() < EPSILON);
    }
}
