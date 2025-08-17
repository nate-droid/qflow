use serde::Deserialize;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum Gate {
    I { qubit: usize },
    H { qubit: usize },
    X { qubit: usize },
    Y { qubit: usize },
    Z { qubit: usize },
    CX { control: usize, target: usize },
    CNOT { control: usize, target: usize }, // Alias for CX
    RX { qubit: usize, theta: f64 },        // target and theta
    RY { qubit: usize, theta: f64 },        // target and theta
    RZ { qubit: usize, theta: f64 },        // target and theta
    Measure,
}

impl Display for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gate::I { qubit } => write!(f, "I q[{}]", qubit),
            Gate::H { qubit } => write!(f, "H q[{}]", qubit),
            Gate::X { qubit } => write!(f, "X q[{}]", qubit),
            Gate::Y { qubit } => write!(f, "Y q[{}]", qubit),
            Gate::Z { qubit } => write!(f, "Z q[{}]", qubit),
            Gate::CX { control, target } | Gate::CNOT { control, target } => {
                write!(f, "CX q[{}],q[{}]", control, target)
            }
            Gate::RX { qubit, theta } => write!(f, "RX q[{}],{}", qubit, theta),
            Gate::RY { qubit, theta } => write!(f, "RY q[{}],{}", qubit, theta),
            Gate::RZ { qubit, theta } => write!(f, "RZ q[{}],{}", qubit, theta),
            Gate::Measure => write!(f, "Measure"),
        }
    }
}

impl Gate {
    pub fn target(&self) -> Vec<usize> {
        match self {
            Gate::X { qubit }
            | Gate::Y { qubit }
            | Gate::Z { qubit }
            | Gate::H { qubit }
            | Gate::RX { qubit, .. }
            | Gate::RY { qubit, .. }
            | Gate::RZ { qubit, .. } => vec![*qubit],
            Gate::CX { target, .. } | Gate::CNOT { target, .. } => vec![*target],

            _ => vec![],
        }
    }
}

pub fn parse_qasm(qasm_str: &str) -> (usize, Vec<Gate>) {
    let mut num_qubits = 0;
    let mut gates = Vec::new();
    let mut has_measured = false; // Flag to ensure we only measure once.

    for line in qasm_str.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty()
            || trimmed_line.starts_with("//")
            || trimmed_line.starts_with("OPENQASM")
            || trimmed_line.starts_with("include")
        {
            continue;
        }

        if trimmed_line.starts_with("qreg") {
            if let Some(start) = trimmed_line.find('[') {
                if let Some(end) = trimmed_line.find(']') {
                    if let Ok(n) = trimmed_line[start + 1..end].parse::<usize>() {
                        num_qubits = n;
                    }
                }
            }
        }
        // Explicitly ignore classical register declarations.
        else if trimmed_line.starts_with("creg") {
            continue;
        } else if trimmed_line.starts_with("h ") {
            if let Some(start) = trimmed_line.find('[') {
                if let Some(end) = trimmed_line.find(']') {
                    if let Ok(q) = trimmed_line[start + 1..end].parse::<usize>() {
                        gates.push(Gate::H { qubit: q });
                    }
                }
            }
        } else if trimmed_line.starts_with("x ") {
            if let Some(start) = trimmed_line.find('[') {
                if let Some(end) = trimmed_line.find(']') {
                    if let Ok(q) = trimmed_line[start + 1..end].parse::<usize>() {
                        gates.push(Gate::X { qubit: q });
                    }
                }
            }
        } else if trimmed_line.starts_with("y ") {
            if let Some(start) = trimmed_line.find('[') {
                if let Some(end) = trimmed_line.find(']') {
                    if let Ok(q) = trimmed_line[start + 1..end].parse::<usize>() {
                        gates.push(Gate::Y { qubit: q });
                    }
                }
            }
        } else if trimmed_line.starts_with("z ") {
            if let Some(start) = trimmed_line.find('[') {
                if let Some(end) = trimmed_line.find(']') {
                    if let Ok(q) = trimmed_line[start + 1..end].parse::<usize>() {
                        gates.push(Gate::Z { qubit: q });
                    }
                }
            }
        } else if trimmed_line.starts_with("cx ") {
            let clean_line = trimmed_line.trim_end_matches(';');
            let parts: Vec<&str> = clean_line
                .split(&[' ', ',', '[', ']'][..])
                .filter(|s| !s.is_empty())
                .collect();
            if parts.len() == 5 && parts[0] == "cx" && parts[1] == "q" && parts[3] == "q" {
                if let (Ok(c), Ok(t)) = (parts[2].parse::<usize>(), parts[4].parse::<usize>()) {
                    gates.push(Gate::CX {
                        control: c,
                        target: t,
                    });
                }
            }
        } else if trimmed_line.starts_with("measure") {
            if !has_measured {
                gates.push(Gate::Measure);
                has_measured = true;
            }
        }
    }
    (num_qubits, gates)
}

pub fn infer_qubits_from_gates(gates: Vec<&Gate>) -> usize {
    let mut max_ix: Option<usize> = None;
    let mut bump = |ix: usize| {
        max_ix = Some(max_ix.map_or(ix, |m| m.max(ix)));
    };

    for g in gates {
        match *g {
            Gate::RX { qubit, .. } |
            Gate::RY { qubit, .. } |
            Gate::RZ { qubit, .. } |
            Gate::H  { qubit, .. } => bump(qubit),

            Gate::CNOT { control, target } => { bump(control); bump(target); }

            // If you have other variants touching qubits, add them here.
            _ => {}
        }
    }
    max_ix.map_or(0, |m| m + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qasm_parser_with_measure() {
        let qasm_input = r#"
            OPENQASM 2.0;
            qreg q[2];
            h q[0];
            cx q[0],q[1];
            measure q -> c;
        "#;
        let (num_qubits, gates) = parse_qasm(qasm_input);

        assert_eq!(num_qubits, 2);
        assert_eq!(gates.len(), 3);
        assert_eq!(gates[0], Gate::H { qubit: 0 });
        assert_eq!(
            gates[1],
            Gate::CX {
                control: 0,
                target: 1
            }
        );
        assert_eq!(gates[2], Gate::Measure);
    }
}
