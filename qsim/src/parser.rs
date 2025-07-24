use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gate {
    I(usize),
    H(usize),
    X(usize),
    Y(usize),
    Z(usize),
    CX(usize, usize),
    RX(usize, f64), // target and theta
    RY(usize, f64), // target and theta
    RZ(usize, f64), // target and theta
    Measure,
}

impl Display for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gate::I(q) => write!(f, "I q[{}]", q),
            Gate::H(q) => write!(f, "H q[{}]", q),
            Gate::X(q) => write!(f, "X q[{}]", q),
            Gate::Y(q) => write!(f, "Y q[{}]", q),
            Gate::Z(q) => write!(f, "Z q[{}]", q),
            Gate::CX(c, t) => write!(f, "CX q[{}],q[{}]", c, t),
            Gate::RX(q, theta) => write!(f, "RX q[{}],{}", q, theta),
            Gate::RY(q, theta) => write!(f, "RY q[{}],{}", q, theta),
            Gate::RZ(q, theta) => write!(f, "RZ q[{}],{}", q, theta),
            Gate::Measure => write!(f, "Measure"),
        }
    }
}

impl Gate {
    pub fn target(&self) -> Vec<usize> {
        match self {
            Gate::H(target) | Gate::X(target) | Gate::Y(target) | Gate::Z(target) | Gate::RX(target, ..) | Gate::RY(target, ..) | Gate::RZ(target, ..) => vec![*target],
            Gate::CX(_, target) => vec![*target],
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
                        gates.push(Gate::H(q));
                    }
                }
            }
        } else if trimmed_line.starts_with("x ") {
            if let Some(start) = trimmed_line.find('[') {
                if let Some(end) = trimmed_line.find(']') {
                    if let Ok(q) = trimmed_line[start + 1..end].parse::<usize>() {
                        gates.push(Gate::X(q));
                    }
                }
            }
        } else if trimmed_line.starts_with("y ") {
            if let Some(start) = trimmed_line.find('[') {
                if let Some(end) = trimmed_line.find(']') {
                    if let Ok(q) = trimmed_line[start + 1..end].parse::<usize>() {
                        gates.push(Gate::Y(q));
                    }
                }
            }
        } else if trimmed_line.starts_with("z ") {
            if let Some(start) = trimmed_line.find('[') {
                if let Some(end) = trimmed_line.find(']') {
                    if let Ok(q) = trimmed_line[start + 1..end].parse::<usize>() {
                        gates.push(Gate::Z(q));
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
                    gates.push(Gate::CX(c, t));
                }
            }
        }

        else if trimmed_line.starts_with("measure") {
            if !has_measured {
                gates.push(Gate::Measure);
                has_measured = true;
            }
        }
    }
    (num_qubits, gates)
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
        assert_eq!(gates[0], Gate::H(0));
        assert_eq!(gates[1], Gate::CX(0, 1));
        assert_eq!(gates[2], Gate::Measure);
    }
}
