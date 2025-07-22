pub mod parser;
pub mod simulator;
pub mod state;

pub mod events;
pub mod circuit;

pub use parser::{Gate, parse_qasm};
pub use simulator::run_simulation;
pub use state::StateVector;
pub use simulator::QuantumSimulator;


#[cfg(test)]
mod tests {
    use super::{Gate, QuantumSimulator};
    use std::f64::consts::PI;
    use crate::circuit::Circuit;
    
    const EPSILON: f64 = 1e-10;
    #[test]
    fn test_ry_rotation_to_one() {
        let mut simulator = QuantumSimulator::new(1);
        let mut circuit = Circuit::new();

        circuit.add_gate(Gate::RY(0, PI));

        simulator.apply_circuit(&circuit);

        // State |1> is at index 1
        let prob_1 = simulator.get_probability(1);
        
        assert!((prob_1 - 1.0).abs() < EPSILON);
    }
}