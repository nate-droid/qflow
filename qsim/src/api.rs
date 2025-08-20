use std::collections::HashMap;
// src/api.rs
use crate::StateVector;
use crate::circuit::Circuit;
use crate::statevector_backend::StatevectorSimulator;

/// A lightweight error enum so callers don't rely on your internals.
#[derive(thiserror::Error, Debug)]
pub enum SimError {
    #[error("QASM parse error: {0}")]
    Qasm(String),
    #[error("Invalid qubit index: {0}")]
    Qubit(usize),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Clone, Copy, Debug)]
pub enum Pauli {
    I,
    X,
    Y,
    Z,
}

/// Everything users typically want to do.
pub trait SimulatorApi {
    fn reset(&mut self, num_qubits: usize);
    fn run(&mut self, circuit: &Circuit) -> Result<(), SimError>;
    fn statevector(&self) -> &StateVector;

    /// Measure a single qubit in Z; collapses the state.
    fn measure(&mut self, qubit: usize) -> Result<u8, SimError>;

    /// Non-destructive expectation ⟨ψ|P|ψ⟩ for a Pauli string.
    /// Example: [(Z,0),(X,2)] means Z on q0 ⊗ X on q2, identity elsewhere.
    fn expectation(&self, ops: &[(Pauli, usize)]) -> Result<f64, SimError>;

    /// Sample computational-basis shots without permanently destroying
    /// the original state (implementation can clone internally).
    fn sample(&self, shots: u32) -> Result<std::collections::HashMap<String, u32>, SimError>;
}

// Small helper: absolute diff
fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn bell_state_expectations() {
    // |Φ+> = (|00> + |11>)/√2
    let qasm = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    h q[0];
    cx q[0], q[1];
    "#;

    let circ = Circuit::from_qasm(qasm).expect("qasm parse");
    let mut sim = StatevectorSimulator::new(circ.num_qubits);
    sim.run(&circ).expect("run");

    // <Z⊗Z> = +1, <X⊗X> = +1, <Z⊗I> = 0, <I⊗Z> = 0
    let zz = sim.expectation(&[(Pauli::Z, 0), (Pauli::Z, 1)]).unwrap();
    let xx = sim.expectation(&[(Pauli::X, 0), (Pauli::X, 1)]).unwrap();
    let z1 = sim.expectation(&[(Pauli::Z, 0)]).unwrap();
    let z2 = sim.expectation(&[(Pauli::Z, 1)]).unwrap();

    assert!(approx_eq(zz, 1.0, 1e-9), "ZZ exp was {}", zz);
    assert!(approx_eq(xx, 1.0, 1e-9), "XX exp was {}", xx);
    assert!(approx_eq(z1, 0.0, 1e-9), "Z⊗I exp was {}", z1);
    assert!(approx_eq(z2, 0.0, 1e-9), "I⊗Z exp was {}", z2);
}

#[test]
fn measure_collapses_single_qubit() {
    // Prepare |1> on q[0], |0> on q[1]
    let qasm = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    x q[0];
    "#;

    let circ = Circuit::from_qasm(qasm).expect("qasm parse");
    let mut sim = StatevectorSimulator::new(circ.num_qubits);
    sim.run(&circ).expect("run");

    // Measuring q0 must deterministically return 1
    let m0 = sim.measure(0).unwrap();
    assert_eq!(m0, 1);

    // Measuring q0 again should still be 1 (already collapsed)
    let m0_again = sim.measure(0).unwrap();
    assert_eq!(m0_again, 1);

    // q1 should be 0
    let m1 = sim.measure(1).unwrap();
    assert_eq!(m1, 0);
}

#[test]
fn sampling_plus_state_is_balanced() {
    // |+> on one qubit: H|0> = (|0> + |1>)/√2
    let qasm = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    h q[0];
    "#;

    let circ = Circuit::from_qasm(qasm).expect("qasm parse");
    let mut sim = StatevectorSimulator::new(circ.num_qubits);
    sim.run(&circ).expect("run");

    // Sample many shots and expect ~50/50
    let shots = 4000;
    let counts = sim.sample(shots).expect("sample");

    // Normalize
    let mut p: HashMap<String, f64> = HashMap::new();
    for (k, v) in counts {
        p.insert(k, (v as f64) / (shots as f64));
    }
    let p0 = *p.get("0").unwrap_or(&0.0);
    let p1 = *p.get("1").unwrap_or(&0.0);

    // With 4000 shots, ±0.05 is a very loose bound (~>6σ); this keeps test stable.
    assert!(approx_eq(p0, 0.5, 0.05), "p(0) ~ 0.5, got {}", p0);
    assert!(approx_eq(p1, 0.5, 0.05), "p(1) ~ 0.5, got {}", p1);
}

#[test]
fn can_reuse_simulator_with_reset() {
    // First: prepare |1> on q0
    let qasm1 = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    x q[0];
    "#;

    // Second: Hadamard on fresh single qubit
    let qasm2 = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    h q[0];
    "#;

    let c1 = Circuit::from_qasm(qasm1).unwrap();
    let c2 = Circuit::from_qasm(qasm2).unwrap();

    let mut sim = StatevectorSimulator::new(1);

    sim.run(&c1).unwrap();
    let m = sim.measure(0).unwrap();
    assert_eq!(m, 1);

    // Reuse same instance; run() should reset internally to c2.num_qubits
    sim.run(&c2).unwrap();

    // Expectation <X> on |+> is +1
    let ex = sim.expectation(&[(Pauli::X, 0)]).unwrap();
    assert!(approx_eq(ex, 1.0, 1e-9), "⟨X⟩ was {}", ex);
}
