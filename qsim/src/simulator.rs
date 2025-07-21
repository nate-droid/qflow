use super::parser::{Gate, parse_qasm};
use super::state::StateVector;
use num_complex::Complex;
use std::f64::consts::FRAC_1_SQRT_2;
use std::io::Write;

use crate::events::{Event, GateInfo, MeasurementInfo, SimulationStartInfo, emit_event};

pub trait QuantumGate {
    fn apply(&self, state: &mut [Complex<f64>]);
}

pub struct QuantumSimulator {
    pub num_qubits: usize,
    pub state: StateVector,
}

/// Main simulation runner.
pub fn run_simulation(qasm_input: &str) -> Option<Vec<Event>> {
    let mut events = Vec::new();

    let hadamard = [
        [
            Complex::new(FRAC_1_SQRT_2, 0.0),
            Complex::new(FRAC_1_SQRT_2, 0.0),
        ],
        [
            Complex::new(FRAC_1_SQRT_2, 0.0),
            Complex::new(-FRAC_1_SQRT_2, 0.0),
        ],
    ];
    let pauli_x = [
        [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
        [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
    ];
    let pauli_y = [
        [Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
        [Complex::new(0.0, 1.0), Complex::new(0.0, 0.0)],
    ];
    let pauli_z = [
        [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        [Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
    ];

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
            Gate::H(target) => state.apply_single_qubit_gate(&hadamard, *target),
            Gate::X(target) => state.apply_single_qubit_gate(&pauli_x, *target),
            Gate::Y(target) => state.apply_single_qubit_gate(&pauli_y, *target),
            Gate::Z(target) => state.apply_single_qubit_gate(&pauli_z, *target),
            Gate::CX(control, target) => state.apply_cx(*control, *target),
            Gate::Measure => {
                let result = state.measure_all(&mut rng);
                // We clone the final state to give ownership to the event.
                // This is necessary to avoid returning a reference to a local variable.
                events.push(Event::MeasurementResult(MeasurementInfo {
                    classical_outcome: result,
                    binary_outcome: format!("{:b}", result),
                    final_state_vector: state.clone(),
                }));
                return Some(events); // Simulation ends on measurement.
            }
        }

        // We clone the state vector here to give ownership of the data
        // to the event. This is necessary because the `state` variable
        // is local to this function and cannot be referenced in the return value.
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
        let hadamard = [
            [
                Complex::new(FRAC_1_SQRT_2, 0.0),
                Complex::new(FRAC_1_SQRT_2, 0.0),
            ],
            [
                Complex::new(FRAC_1_SQRT_2, 0.0),
                Complex::new(-FRAC_1_SQRT_2, 0.0),
            ],
        ];
        let mut state = StateVector::new(2);
        state.apply_single_qubit_gate(&hadamard, 0);
        state.apply_cx(0, 1);
        let expected_amp = Complex::new(FRAC_1_SQRT_2, 0.0);
        assert!(approx_eq(state.amplitudes[0], expected_amp));
        assert!(approx_eq(state.amplitudes[1], Complex::new(0.0, 0.0)));
        assert!(approx_eq(state.amplitudes[2], Complex::new(0.0, 0.0)));
        assert!(approx_eq(state.amplitudes[3], expected_amp));
    }
}
