use num_complex::Complex;
use qsim::circuit::{Circuit, circuit_to_qasm};
use qsim::simulator::Simulator;
use qsim::{Gate, QuantumSimulator};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// This allows Rust to log to the browser's developer console.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn error(s: &str);
}

// --- Data Structures for Communication with JavaScript ---

/// Represents a single quantum gate. `serde(tag = "type")` ensures that
/// the JSON representation matches the object structure in React (e.g., { type: "H", ... }).
// #[derive(Serialize, Deserialize, Debug)]
// #[serde(tag = "type")]
// enum Gate {
//     H { qubit: usize },
//     X { qubit: usize },
//     Y { qubit: usize },
//     Z { qubit: usize },
//     CNOT { control: usize, target: usize },
// }

/// Represents the entire circuit, matching the structure sent from the React frontend.
// #[derive(Serialize, Deserialize, Debug)]
// #[serde(rename_all = "camelCase")]
// struct Circuit {
//     num_qubits: usize,
//     moments: Vec<Vec<Gate>>,
// }

/// Represents the final results of the simulation to be sent back to JavaScript.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SimulationResult {
    /// The final state vector as a list of complex numbers (real, imaginary).
    state_vector: Vec<(f64, f64)>,
    /// The probability of measuring each basis state.
    probabilities: Vec<f64>,
}

// --- Core Simulation Logic ---

/// The main simulation engine.
fn run_simulation_engine(circuit: Circuit) -> SimulationResult {
    let num_qubits = circuit.num_qubits;
    let num_states = 1 << num_qubits; // 2^n

    let mut sim = QuantumSimulator::new(num_qubits);
    // Initialize state vector to |0...0>, which is [1, 0, 0, ...].

    let mut state_vector: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); num_states];
    state_vector[0] = Complex::new(1.0, 0.0);

    // Apply each gate in each moment.
    for moment in circuit.moments {
        for gate in moment {
            // apply_gate(&mut state_vector, &gate, num_qubits);
            sim.apply_gate(&gate);
        }
    }

    // Calculate final probabilities from the amplitudes.
    // let probabilities: Vec<f64> = state_vector.iter().map(|c| c.norm_sqr()).collect();
    let probabilities: Vec<f64> = sim.get_statevector().iter().map(|c| c.norm_sqr()).collect();

    // Convert complex numbers to a serializable tuple format (real, imag).
    let serializable_state_vector = sim.get_statevector().iter().map(|c| (c.re, c.im)).collect();

    SimulationResult {
        state_vector: serializable_state_vector,
        probabilities,
    }
}

/// Applies a generic 2x2 matrix to a specific qubit.
fn apply_single_qubit_gate(
    state_vector: &mut Vec<Complex<f64>>,
    qubit: usize,
    matrix: &[[Complex<f64>; 2]; 2],
    num_qubits: usize,
) {
    let stride = 1 << qubit;
    let num_states = 1 << num_qubits;

    for i in 0..num_states {
        // Check if the i-th bit of the index matches the qubit's influence.
        if (i >> qubit) & 1 == 0 {
            let i0 = i;
            let i1 = i + stride;

            let psi0 = state_vector[i0];
            let psi1 = state_vector[i1];

            state_vector[i0] = matrix[0][0] * psi0 + matrix[0][1] * psi1;
            state_vector[i1] = matrix[1][0] * psi0 + matrix[1][1] * psi1;
        }
    }
}

/// Applies the CNOT gate.
fn apply_cnot(
    state_vector: &mut Vec<Complex<f64>>,
    control: usize,
    target: usize,
    num_qubits: usize,
) {
    let num_states = 1 << num_qubits;
    for i in 0..num_states {
        // Check if the control bit is 1.
        if (i >> control) & 1 == 1 {
            // If control is 1, flip the target bit. This means swapping amplitudes.
            let target_mask = 1 << target;
            let i0 = i & !target_mask; // State where target is 0
            let i1 = i | target_mask; // State where target is 1

            // We only need to swap once, so we only do it when i corresponds to the state
            // where the target bit is 0.
            if (i >> target) & 1 == 0 {
                state_vector.swap(i0, i1);
            }
        }
    }
}

// --- Gate Matrix Definitions ---
struct GateMatrix;
impl GateMatrix {
    const H: [[Complex<f64>; 2]; 2] = [
        [
            Complex {
                re: 0.70710678,
                im: 0.0,
            },
            Complex {
                re: 0.70710678,
                im: 0.0,
            },
        ],
        [
            Complex {
                re: 0.70710678,
                im: 0.0,
            },
            Complex {
                re: -0.70710678,
                im: 0.0,
            },
        ],
    ];
    const X: [[Complex<f64>; 2]; 2] = [
        [Complex { re: 0.0, im: 0.0 }, Complex { re: 1.0, im: 0.0 }],
        [Complex { re: 1.0, im: 0.0 }, Complex { re: 0.0, im: 0.0 }],
    ];
    const Y: [[Complex<f64>; 2]; 2] = [
        [Complex { re: 0.0, im: 0.0 }, Complex { re: 0.0, im: -1.0 }],
        [Complex { re: 0.0, im: 1.0 }, Complex { re: 0.0, im: 0.0 }],
    ];
    const Z: [[Complex<f64>; 2]; 2] = [
        [Complex { re: 1.0, im: 0.0 }, Complex { re: 0.0, im: 0.0 }],
        [Complex { re: 0.0, im: 0.0 }, Complex { re: -1.0, im: 0.0 }],
    ];
}

// --- WASM Export ---

/// The public function that will be callable from JavaScript.
/// It takes a JSON string representing the circuit and returns a JSON string
/// with the simulation results.
#[wasm_bindgen]
pub fn run_simulation(circuit_json: &str) -> String {
    // Deserialize the input string into our Rust `Circuit` struct.
    let circuit: Circuit = match serde_json::from_str(circuit_json) {
        Ok(c) => c,
        Err(e) => {
            error(&format!("Error deserializing circuit: {}", e));
            // Return a JSON object indicating the error.
            return serde_json::json!({ "error": format!("Failed to parse circuit: {}", e) })
                .to_string();
        }
    };

    // Run the simulation.
    let result = run_simulation_engine(circuit);

    // Serialize the `SimulationResult` struct back into a JSON string.
    serde_json::to_string(&result).unwrap_or_else(|e| {
        error(&format!("Error serializing result: {}", e));
        serde_json::json!({ "error": format!("Failed to serialize result: {}", e) }).to_string()
    })
}

#[wasm_bindgen]
pub fn compile_circuit_to_qasm(circuit_json: &str) -> String {
    // Deserialize the input string into our Rust `Circuit` struct.
    let circuit: Circuit = match serde_json::from_str(circuit_json) {
        Ok(c) => c,
        Err(e) => {
            error(&format!("Error deserializing circuit: {}", e));
            // Return a JSON object indicating the error.
            return serde_json::json!({ "error": format!("Failed to parse circuit: {}", e) })
                .to_string();
        }
    };

    // Convert the circuit to QASM format.
    let qasm = circuit_to_qasm(&circuit);

    // Return the QASM string.
    qasm
}
