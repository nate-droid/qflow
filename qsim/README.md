# qsim

qsim is a minimal quantum computer simulator. It has been designed to be unaware of the other components, 
so can be used as a standalone simulator. It is used by the QFlow operator to run quantum circuits.

You can run the provided examples (from the root directory) with:

```bash
cargo run --bin qsim -- --input-file examples/bell.qasm --output-file results.json
```

Alternatively, you can write a quantum circuit in Rust using the `qsim` crate and run it directly.

# Example Rust Code

```rust
use qsim::circuit::Circuit;
use qsim::gates::{H, CNOT};

fn main() {
    // Create a new circuit with 2 qubits
    let mut circuit = Circuit::new(2);

    // Apply a Hadamard gate to the first qubit
    circuit.apply_gate(H(0));

    // Apply a CNOT gate with control on the first qubit and target on the second qubit
    circuit.apply_gate(CX(0, 1));

    // Measure the qubits and print the results
    let results = circuit.measure();
    println!("Measurement results: {:?}", results);
}
```