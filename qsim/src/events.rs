use crate::state::StateVector;
use serde::Serialize;
use std::io::Write;

/// Top-level enum to distinguish between different event types.
#[derive(Serialize)]
#[serde(tag = "eventType")]
pub enum Event {
    SimulationStart(SimulationStartInfo),
    GateApplication(GateInfo),
    MeasurementResult(MeasurementInfo),
}

/// Information about the start of a simulation.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationStartInfo {
    pub num_qubits: usize,
    pub num_gates: usize,
}

/// Information about a single gate application step.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateInfo {
    pub step: usize,
    pub gate: String,
    pub state_vector: StateVector,
}

/// Information about the final measurement.
#[derive(Serialize)]
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
