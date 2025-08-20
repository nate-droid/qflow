// src/facade.rs
use crate::StateVector;
use crate::api::{Pauli, SimError, SimulatorApi};
use crate::circuit::Circuit;
use crate::statevector_backend::StatevectorSimulator;

pub fn run_qasm_return_statevector(qasm: &str) -> Result<StateVector, SimError> {
    let circ = Circuit::from_qasm(qasm)?;
    let mut sim = StatevectorSimulator::new(circ.num_qubits);
    sim.run(&circ)?;
    Ok(sim.statevector().clone())
}

pub fn run_qasm_expectation(qasm: &str, ops: &[(Pauli, usize)]) -> Result<f64, SimError> {
    let circ = Circuit::from_qasm(qasm)?;
    let mut sim = StatevectorSimulator::new(circ.num_qubits);
    sim.run(&circ)?;
    sim.expectation(ops)
}

pub fn run_qasm_measure(qasm: &str, qubit: usize) -> Result<u8, SimError> {
    let circ = Circuit::from_qasm(qasm)?;
    let mut sim = StatevectorSimulator::new(circ.num_qubits);
    sim.run(&circ)?;
    sim.measure(qubit)
}

pub fn run_qasm_counts(
    qasm: &str,
    shots: u32,
) -> Result<std::collections::HashMap<String, u32>, SimError> {
    let circ = Circuit::from_qasm(qasm)?;
    let mut sim = StatevectorSimulator::new(circ.num_qubits);
    sim.run(&circ)?;
    sim.sample(shots)
}
