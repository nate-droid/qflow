use crate::{parse_qasm, Gate};
use serde::Deserialize;
use std::fmt;
use crate::api::SimError;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Circuit {
    pub num_qubits: usize,
    pub moments: Vec<Vec<Gate>>,
}

impl Circuit {
    pub fn new() -> Self {
        Self {
            num_qubits: 0,
            moments: Vec::new(),
        }
    }

    pub fn with_qubits(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            moments: Vec::new(),
        }
    }

    pub fn add_gate(&mut self, gate: Gate) {
        self.moments.push(vec![gate]);
    }

    pub fn add_moment(&mut self, gates: Vec<Gate>) {
        self.moments.push(gates);
    }

    pub fn num_moments(&self) -> usize {
        self.moments.len()
    }

    pub fn set_num_qubits(&mut self, num_qubits: usize) {
        self.num_qubits = num_qubits;
    }

    pub fn moments(&self) -> &Vec<Vec<Gate>> {
        &self.moments
    }

    pub fn gates_flat(&self) -> Vec<&Gate> {
        self.moments.iter().flat_map(|m| m.iter()).collect()
    }

    pub fn from_qasm(src: &str) -> Result<Self, SimError> {
        let (num_qubits, gates) = parse_qasm(src);
        let mut c = Circuit::with_qubits(num_qubits);
        // Put each gate in its own moment by default (keeps ordering simple)
        for g in gates { c.add_moment(vec![g]); }
        Ok(c)
    }
}
impl fmt::Display for Circuit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.num_qubits == 0 {
            return write!(f, "Empty Circuit");
        }

        // 1. Create a grid to place gate symbols.
        // The grid dimensions are num_qubits x num_moments.
        let num_moments = self.moments.len();
        let mut grid = vec![vec![String::from("───"); num_moments]; self.num_qubits];

        // 2. Populate the grid with gate representations.
        for (moment_idx, moment) in self.moments.iter().enumerate() {
            for gate in moment {
                match *gate {
                    Gate::H { qubit } => grid[qubit][moment_idx] = "[H]".to_string(),
                    Gate::X { qubit } => grid[qubit][moment_idx] = "[X]".to_string(),
                    Gate::CX { control, target } | Gate::CNOT { control, target } => {
                        grid[control][moment_idx] = "─●─".to_string();
                        grid[target][moment_idx] = "─⊕─".to_string();
                        let start = control.min(target);
                        let end = control.max(target);
                        for i in (start + 1)..end {
                            grid[i][moment_idx] = " │ ".to_string();
                        }
                    }
                    Gate::Y { qubit } => grid[qubit][moment_idx] = "[Y]".to_string(),
                    Gate::Z { qubit } => grid[qubit][moment_idx] = "[Z]".to_string(),
                    _ => {
                        panic!("Unknown gate {:?}", gate);
                    }
                }
            }
        }

        // 3. Render the grid into the final string.
        let mut output = String::new();
        for qubit_idx in 0..self.num_qubits {
            output.push_str(&format!("q{}: ", qubit_idx));
            for moment_idx in 0..num_moments {
                output.push_str(&grid[qubit_idx][moment_idx]);
            }
            output.push('\n');
        }
        write!(f, "{}", output)
    }
}

// this is a naive implementation, and does nothing to optimize the circuit (yet)
pub fn gates_to_circuit(gates: Vec<Gate>) -> Circuit {
    let mut circuit = Circuit::new();

    // Determine the number of qubits based on the highest qubit index in the gates
    let mut highest_qubit = 0;

    for gate in gates {
        circuit.add_gate(gate);

        // iterate through the targets of the gate to find the highest qubit index
        for qubit in gate.target() {
            if qubit > highest_qubit {
                highest_qubit = qubit;
            }
        }
    }

    circuit.set_num_qubits(highest_qubit + 1); // +1 because qubits are 0-indexed

    circuit
}

pub fn circuit_to_qasm(circuit: &Circuit) -> String {
    let mut qasm = String::new();
    qasm.push_str("OPENQASM 2.0;\n");
    qasm.push_str("include \"qelib1.inc\";\n");
    qasm.push_str(&format!("qreg q[{}];\n", circuit.num_qubits));

    for moment in &circuit.moments {
        for gate in moment {
            match gate {
                Gate::H { qubit } => qasm.push_str(&format!("H q[{}];\n", qubit)),
                Gate::X { qubit } => qasm.push_str(&format!("X q[{}];\n", qubit)),
                Gate::Y { qubit } => qasm.push_str(&format!("Y q[{}];\n", qubit)),
                Gate::Z { qubit } => qasm.push_str(&format!("Z q[{}];\n", qubit)),
                Gate::RX { qubit, theta } => {
                    qasm.push_str(&format!("RX q[{}], {};\n", qubit, theta))
                }
                Gate::RY { qubit, theta } => {
                    qasm.push_str(&format!("RY q[{}], {};\n", qubit, theta))
                }
                Gate::RZ { qubit, theta } => {
                    qasm.push_str(&format!("RZ q[{}], {};\n", qubit, theta))
                }
                Gate::CX { control, target } | Gate::CNOT { control, target } => {
                    qasm.push_str(&format!("CX q[{}],q[{}];\n", control, target));
                }
                _ => panic!("Unsupported gate type: {:?}", gate),
            }
        }
    }
    qasm
}

// tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gate;

    #[test]
    fn test_circuit_display() {
        let mut circuit = Circuit::new();
        circuit.num_qubits = 2;
        circuit.add_moment(vec![Gate::H { qubit: 0 }]);
        circuit.add_moment(vec![Gate::CX {
            control: 0,
            target: 1,
        }]);
        circuit.add_moment(vec![Gate::X { qubit: 1 }]);

        let expected_output = "q0: [H]─●────\nq1: ────⊕─[X]\n";
        assert_eq!(format!("{}", circuit), expected_output);
        println!("{}", circuit);
    }

    #[test]
    fn test_gates_to_circuit() {
        let gates = vec![
            Gate::H { qubit: 0 },
            Gate::CX {
                control: 0,
                target: 1,
            },
            Gate::X { qubit: 1 },
        ];
        let circuit = gates_to_circuit(gates);
        assert_eq!(circuit.num_moments(), 3);
        assert_eq!(circuit.num_qubits, 2);
    }

    #[test]
    fn circuit_to_qasm_test() {
        let mut circuit = Circuit::new();
        circuit.num_qubits = 2;
        circuit.add_moment(vec![Gate::H { qubit: 0 }]);
        circuit.add_moment(vec![Gate::CX {
            control: 0,
            target: 1,
        }]);
        circuit.add_moment(vec![Gate::X { qubit: 1 }]);

        let qasm = circuit_to_qasm(&circuit);
        let expected_qasm =
            "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[2];\nH q[0];\nCX q[0],q[1];\nX q[1];\n";
        assert_eq!(qasm, expected_qasm);
    }
}
