use std::fmt;
use serde::Deserialize;
use crate::Gate;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Circuit {
    pub num_qubits: usize,
    pub moments: Vec<Vec<Gate>>,
}

impl Circuit {
    pub fn new() -> Self {
        Self { num_qubits: 0, moments: Vec::new() }
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
                    Gate::H{qubit} => grid[qubit][moment_idx] = "[H]".to_string(),
                    Gate::X{qubit} => grid[qubit][moment_idx] = "[X]".to_string(),
                    Gate::CX{control, target} | Gate::CNOT {control, target} => {
                        grid[control][moment_idx] = "─●─".to_string();
                        grid[target][moment_idx] = "─⊕─".to_string();
                        let start = control.min(target);
                        let end = control.max(target);
                        for i in (start + 1)..end {
                            grid[i][moment_idx] = " │ ".to_string();
                        }
                    }
                    Gate::Y{qubit} => grid[qubit][moment_idx] = "[Y]".to_string(),
                    Gate::Z{qubit} => grid[qubit][moment_idx] = "[Z]".to_string(),
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
    for gate in gates {
        circuit.add_gate(gate);
    }
    circuit
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
        circuit.add_moment(vec![Gate::H{qubit: 0}]);
        circuit.add_moment(vec![Gate::CX{control: 0, target: 1}]);
        circuit.add_moment(vec![Gate::X{qubit: 1}]);

        let expected_output = "q0: [H]─●────\nq1: ────⊕─[X]\n";
        assert_eq!(format!("{}", circuit), expected_output);
        println!("{}", circuit);
    }
}
