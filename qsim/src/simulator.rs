use super::parser::{Gate, parse_qasm};
use super::state::StateVector;
use num_complex::Complex;
use std::f64::consts::FRAC_1_SQRT_2;
use crate::circuit::Circuit;
use crate::events::{Event, GateInfo, MeasurementInfo, SimulationStartInfo, emit_event};

pub trait QuantumGate {
    fn apply(&self, state: &mut [Complex<f64>]);
}

pub struct QuantumSimulator {
    pub num_qubits: usize,
    pub state: StateVector,
}

impl QuantumSimulator {
    pub fn new(num_qubits: usize) -> Self {
        QuantumSimulator {
            num_qubits,
            state: StateVector::new(num_qubits),
        }
    }

    pub fn reset(&mut self) {
        self.state.reset();
    }

    pub fn apply_circuit(&mut self, circuit: &Circuit) {
        for gate in &circuit.gates {
            self.apply_gate(gate);
        }
    }

    pub fn apply_gate(&mut self, gate: &Gate) {
        match gate {
            Gate::H(target) => self.state.apply_single_qubit_gate(&HADAMARD, *target),
            Gate::X(target) => self.state.apply_single_qubit_gate(&PAULI_X, *target),
            Gate::Y(target) => self.state.apply_single_qubit_gate(&PAULI_Y, *target),
            Gate::Z(target) => self.state.apply_single_qubit_gate(&PAULI_Z, *target),
            Gate::CX(control, target) => self.state.apply_cx(*control, *target),
            Gate::Measure => {
                let result = self.state.measure_all(&mut rand::thread_rng());
            }
            _ => {
                let matrix = construct_gate_matrix(gate);
                
                if let Some(matrix) = matrix {
                    if gate.target().len() == 1 {
                        self.state.apply_single_qubit_gate(&matrix, gate.target()[0]);
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

    pub fn get_probability(&self, state_index: usize) -> f64 {
        if state_index >= self.state.amplitudes.len() {
            eprintln!("Error: State index out of bounds.");
            return 0.0;
        }
        let amp = self.state.amplitudes[state_index];
        (amp.re * amp.re + amp.im * amp.im).sqrt()
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
        Gate::RX(qubit, theta) => {
            Some([
                [
                    Complex::new((theta / 2.0).cos(), 0.0),
                    Complex::new(0.0, -(theta / 2.0).sin()),
                ],
                [
                    Complex::new(0.0, -(theta / 2.0).sin()),
                    Complex::new((theta / 2.0).cos(), 0.0),
                ],
            ])
        }
        Gate::RY(qubit, theta) => {
            Some([
                [
                    Complex::new((theta / 2.0).cos(), 0.0),
                    Complex::new(0.0, -(theta / 2.0).sin()),
                ],
                [
                    Complex::new(0.0, (theta / 2.0).sin()),
                    Complex::new((theta / 2.0).cos(), 0.0),
                ],
            ])

        }
        _ => {
            eprintln!("Unsupported gate type: {:?}", gate);
            panic!("Unsupported gate type encountered during simulation.");
        }, // Unsupported gate type
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
            Gate::H(target) => state.apply_single_qubit_gate(&HADAMARD, *target),
            Gate::X(target) => state.apply_single_qubit_gate(&PAULI_X, *target),
            Gate::Y(target) => state.apply_single_qubit_gate(&PAULI_Y, *target),
            Gate::Z(target) => state.apply_single_qubit_gate(&PAULI_Z, *target),
            Gate::CX(control, target) => state.apply_cx(*control, *target),
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
