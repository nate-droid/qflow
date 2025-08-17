use crate::state::StateVector;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize, Debug)]
#[serde(tag = "eventType")]
pub enum Event {
    SimulationStart(SimulationStartInfo),
    GateApplication(GateInfo),
    MeasurementResult(MeasurementInfo),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SimulationStartInfo {
    pub num_qubits: usize,
    pub num_gates: usize,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GateInfo {
    pub step: usize,
    pub gate: String,
    pub state_vector: StateVector,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MeasurementInfo {
    pub classical_outcome: usize,
    pub binary_outcome: String,
    pub final_state_vector: StateVector,
}

/// Helper function to serialize and print an event to a writer.
pub fn emit_event(event: &Event, writer: &mut impl Write) {
    let json_output = serde_json::to_string(event).expect("Failed to serialize event to JSON.");
    writeln!(writer, "{}", json_output).expect("Failed to write to output.");
}
