#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gate {
    H(usize),
    X(usize),
    Y(usize),
    Z(usize),
    CX(usize, usize),
    RX(usize, f64), // RX gate with angle
    RY(usize, f64), // RY gate with angle
    RZ(usize, f64), // RZ gate with angle
    Measure,
}

/// A very simple OpenQASM parser.
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
        // Since our simulator only supports one "measure all" operation,
        // we'll treat the first measure instruction we see as the trigger
        // and ignore any subsequent ones.
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
