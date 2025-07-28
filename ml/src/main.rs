mod integrations;

use std::str::FromStr;
use num_complex::Complex;
use qsim::{Gate, QuantumSimulator};
use qsim::simulator::Simulator;

/// Parses a circuit string into a vector of Gate objects.
/// This function is the bridge between the string representation and the simulator.
fn parse_circuit(circuit_str: &str) -> Result<Vec<qsim::Gate>, String> {
    let mut gates = Vec::new();
    for line in circuit_str.lines() {
        let trimmed_line = line.trim();
        // Skip metadata, comments, declarations, and empty lines
        if trimmed_line.is_empty()
            || trimmed_line.starts_with("OPENQASM")
            || trimmed_line.starts_with("include")
            || trimmed_line.starts_with("//")
            || trimmed_line.starts_with("qreg")
            || trimmed_line.starts_with("creg")
            || trimmed_line.starts_with("measure")
        {
            continue;
        }

        // Clean up the line by removing the trailing semicolon
        let clean_line = trimmed_line.trim_end_matches(';').trim();

        // Split the line into the operation part and the qubit part
        let mut parts = clean_line.splitn(2, ' ');
        let op_str = parts.next().ok_or_else(|| format!("Missing operation in line: '{}'", clean_line))?;
        let qubit_args_str = parts.next().ok_or_else(|| format!("Missing qubit arguments in line: '{}'", clean_line))?;

        // --- Parse Qubit Arguments ---
        let qubit_indices: Vec<usize> = qubit_args_str
            .split(',')
            .map(|s| {
                s.trim()
                    .trim_start_matches("q[")
                    .trim_end_matches(']')
                    .parse::<usize>()
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Invalid qubit index in '{}': {}", qubit_args_str, e))?;

        // --- Parse Operation ---
        let (name, parameter) = if op_str.contains('(') {
            // It's a parameterized gate like rz(angle)
            let op_parts: Vec<&str> = op_str.split(|c| c == '(' || c == ')').collect();
            if op_parts.len() < 2 || op_parts[1].is_empty() {
                return Err(format!("Invalid parameterized gate format: {}", op_str));
            }
            let param = f64::from_str(op_parts[1])
                .map_err(|e| format!("Invalid gate parameter in '{}': {}", op_str, e))?;
            (op_parts[0].to_string(), Some(param))
        } else {
            // It's a non-parameterized gate like h or cx
            (op_str.to_string(), None)
        };

        // --- Create Gate Struct(s) ---
        match name.as_str() {
            "cx" => {
                if qubit_indices.len() != 2 {
                    return Err(format!("CX gate requires 2 qubits, found {}: '{}'", qubit_indices.len(), clean_line));
                }
                gates.push(Gate::CX(0, 1));
            }
            "rz" => {
                if qubit_indices.len() != 1 {
                    return Err(format!("Rz gate requires 1 qubit, found {}: '{}'", qubit_indices.len(), clean_line));
                }
                if let Some(param) = parameter {
                    gates.push(Gate::RZ(qubit_indices[0], param));
                } else {
                    return Err(format!("Rz gate requires a parameter, found none in '{}'", clean_line));
                }
            }
            "h" => {
                if qubit_indices.len() != 1 {
                    return Err(format!("Hadamard gate requires 1 qubit, found {}: '{}'", qubit_indices.len(), clean_line));
                }
                gates.push(Gate::H(qubit_indices[0]));;
            }
            _ => { // For single-qubit gates like h, rz, etc.
                panic!("Invalid gate specified: '{}'", name);
            }
        }
    }
    Ok(gates)
}

/// Computes the kernel value (fidelity) between two quantum-encoded data points.
///
/// This function simulates the process of:
/// 1. Encoding classical data points `point_a` and `point_b` into quantum circuits.
/// 2. Running these circuits on a quantum simulator to get their final statevectors.
/// 3. Calculating the fidelity between the two statevectors, which is the squared
///    inner product: |<state_a|state_b>|^2.
///
/// # Arguments
/// * `point_a` - A slice of f64 representing the first classical data point.
/// * `point_b` - A slice of f64 representing the second classical data point.
///
/// # Returns
/// A single f64 value between 0.0 and 1.0 representing the similarity.
pub fn compute_kernel_value(point_a: &[f64], point_b: &[f64]) -> f64 {
    // For this example, we assume the number of qubits is determined by the data dimension.
    // A more robust implementation would handle mismatched dimensions.
    assert_eq!(point_a.len(), point_b.len(), "Data points must have the same dimension.");
    let num_qubits = point_a.len();
    if num_qubits == 0 { return 1.0; } // Handle empty data points
    let mut simulator = QuantumSimulator::new(num_qubits);

    // --- Step 1: Simulate the circuit for point_a ---
    let circuit_str_a = create_encoding_circuit(point_a);
    // Parse the string into executable gates. Handle potential errors.
    let circuit_a = parse_circuit(&circuit_str_a).expect("Failed to parse circuit A");

    simulator.reset(); // Start from the |0...0> state
    for gate in &circuit_a {
        simulator.apply_gate(gate);
    }
    let statevector_a = simulator.get_statevector().clone();

    // --- Step 2: Simulate the circuit for point_b ---
    let circuit_str_b = create_encoding_circuit(point_b);
    let circuit_b = parse_circuit(&circuit_str_b).expect("Failed to parse circuit B");

    simulator.reset(); // Reset for the second simulation
    for gate in &circuit_b {
        simulator.apply_gate(gate);
    }
    let statevector_b = simulator.get_statevector().clone();

    // --- Step 3: Calculate the fidelity ---
    // The inner product <a|b> is the sum of (a_i^* * b_i).
    let inner_product: Complex<f64> = statevector_a
        .iter()
        .zip(statevector_b.iter())
        .map(|(a, b)| a.conj() * b)
        .sum();

    // The fidelity is the squared magnitude of the inner product.
    inner_product.norm_sqr()
}

/// This module contains the functionality to create a quantum circuit
/// for encoding classical data using a ZZ Feature Map.

/// Creates an OpenQASM 2.0 string representing a quantum circuit that
/// encodes a 2D classical data point using a ZZ Feature Map.
///
/// The ZZ Feature Map is a common technique in Quantum Machine Learning
/// to encode classical data into the quantum state of a circuit.
/// This implementation uses a 2-qubit circuit.
///
/// The encoding process is as follows:
/// 1. Start with a 2-qubit register.
/// 2. Apply a Hadamard gate to each qubit to create a superposition.
/// 3. Apply parameterized single-qubit rotations (Rz) based on the input data.
/// 4. Apply an entangling gate (CNOT) between the qubits.
/// 5. Apply another parameterized single-qubit rotation (Rz) that is a
///    function of the product of the two data features. This captures the
///    "ZZ" interaction.
/// 6. Apply another layer of Hadamard gates.
///
/// # Arguments
///
/// * `data_point` - A slice of f64 with two elements, e.g., `&[x, y]`.
///
/// # Returns
///
/// A `String` containing the OpenQASM 2.0 representation of the circuit.
///
/// # Panics
///
/// This function will panic if the input `data_point` does not contain exactly 2 elements.
fn create_encoding_circuit(data_point: &[f64]) -> String {
    // Ensure the data point is 2-dimensional for our 2-qubit feature map.
    if data_point.len() != 2 {
        panic!("This function requires a 2D data point, e.g., [x, y].");
    }

    let x = data_point[0];
    let y = data_point[1];

    // The angles for the Rz gates are derived from the input data.
    // A common practice is to scale the data, here we use it directly for simplicity.
    let angle_x = x * std::f64::consts::PI;
    let angle_y = y * std::f64::consts::PI;

    // The angle for the ZZ interaction term.
    // This is often a non-linear combination of the inputs.
    // A simple choice is (pi - x) * (pi - y)
    let angle_zz = (std::f64::consts::PI - x) * (std::f64::consts::PI - y);

    // Using a mutable string to build the OpenQASM code.
    let mut qasm_string = String::new();

    // --- OpenQASM Header ---
    qasm_string.push_str("OPENQASM 2.0;\n");
    qasm_string.push_str("include \"qelib1.inc\";\n\n");

    // --- Qubit and Classical Register Declaration ---
    qasm_string.push_str("// Declare a 2-qubit register for the feature map\n");
    qasm_string.push_str("qreg q[2];\n");
    qasm_string.push_str("// Declare a 2-bit classical register for measurement (optional, for simulation)\n");
    qasm_string.push_str("creg c[2];\n\n");

    // --- Circuit Implementation ---

    // 1. Apply Hadamard gates to all qubits to create superposition
    qasm_string.push_str("// Step 1: Create superposition\n");
    qasm_string.push_str("h q[0];\n");
    qasm_string.push_str("h q[1];\n\n");

    // 2. Apply first layer of parameterized rotations
    qasm_string.push_str("// Step 2: Encode data features with Rz gates\n");
    qasm_string.push_str(&format!("rz({}) q[0];\n", angle_x));
    qasm_string.push_str(&format!("rz({}) q[1];\n", angle_y));
    qasm_string.push_str("\n");

    // 3. Apply entangling gates (linear entanglement)
    qasm_string.push_str("// Step 3: Entangle qubits with CNOT gates\n");
    qasm_string.push_str("cx q[0], q[1];\n\n");

    // 4. Apply the ZZ interaction term
    qasm_string.push_str("// Step 4: Apply the ZZ interaction term\n");
    qasm_string.push_str(&format!("rz({}) q[1];\n\n", angle_zz));

    // 5. Un-entangle with another CNOT
    qasm_string.push_str("// Step 5: Un-entangle\n");
    qasm_string.push_str("cx q[0], q[1];\n\n");

    // --- Optional: Add measurements to observe the final state ---
    qasm_string.push_str("// Optional: Measure qubits\n");
    qasm_string.push_str("measure q -> c;\n");

    // strip the

    qasm_string
}

/// Main function to demonstrate the usage of `create_encoding_circuit`.
fn main() {
    // Example data point from a source like `make_circles`.
    let data_point = [0.5, 0.8];

    // Generate the OpenQASM string for the data point.
    let qasm_circuit = create_encoding_circuit(&data_point);

    // Print the generated circuit.
    println!("--- Generated OpenQASM 2.0 Circuit ---");
    println!("{}", qasm_circuit);

    // Kernel computation example
    let data_point_1 = vec![0.5, 0.2];
    let data_point_2 = vec![0.55, 0.25]; // A point very close to the first one
    let data_point_3 = vec![-0.8, 0.9];  // A point far away

    println!("--- Quantum Kernel Similarity ---");
    println!("Point 1: {:?}", data_point_1);
    println!("Point 2: {:?}", data_point_2);
    println!("Point 3: {:?}", data_point_3);
    println!("---------------------------------");

    // Calculate the kernel value (similarity) between point 1 and itself.
    // This should be 1.0, as a state is perfectly similar to itself.
    let similarity_1_1 = compute_kernel_value(&data_point_1, &data_point_1);
    println!("Similarity(Point 1, Point 1): {:.6}", similarity_1_1);

    // Calculate the similarity between two nearby points.
    // This should result in a high value, close to 1.0.
    let similarity_1_2 = compute_kernel_value(&data_point_1, &data_point_2);
    println!("Similarity(Point 1, Point 2): {:.6}", similarity_1_2);

    // Calculate the similarity between two distant points.
    // This should result in a lower value.
    let similarity_1_3 = compute_kernel_value(&data_point_1, &data_point_3);
    println!("Similarity(Point 1, Point 3): {:.6}", similarity_1_3);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_gate() {
        let qasm = "h q[0];";
        let gates = parse_circuit(qasm).unwrap();
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0], Gate::H(0));
    }

    #[test]
    fn test_parse_parameterized_gate() {
        let qasm = "rz(1.57) q[1];";
        let gates = parse_circuit(qasm).unwrap();
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0], Gate::RZ(1, 1.57));
    }

    #[test]
    fn test_parse_two_qubit_gate() {
        let qasm = "cx q[0], q[1];";
        let gates = parse_circuit(qasm).unwrap();
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0], Gate::CX(0, 1));
    }

    #[test]
    fn test_empty_and_comment_only_input() {
        assert!(parse_circuit("").unwrap().is_empty());
        let qasm_comments = r#"
            // This is a circuit with only comments
            // and blank lines.
        "#;
        assert!(parse_circuit(qasm_comments).unwrap().is_empty());
    }

    #[test]
    fn test_invalid_qubit_format() {
        let qasm = "h q[a];";
        assert!(parse_circuit(qasm).is_err());
    }

    #[test]
    fn test_cx_gate_wrong_qubit_count() {
        let qasm = "cx q[0];";
        assert!(parse_circuit(qasm).is_err());
        let qasm2 = "cx q[0], q[1], q[2];";
        assert!(parse_circuit(qasm2).is_err());
    }

    #[test]
    fn test_parameterized_gate_invalid_param() {
        let qasm = "rz(not_a_number) q[0];";
        assert!(parse_circuit(qasm).is_err());
    }
}