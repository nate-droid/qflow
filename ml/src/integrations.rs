use ndarray::ArrayView1;
use numpy::PyReadonlyArray1;
use pyo3::prelude::*;
use qsim::QuantumSimulator;
use qsim::simulator::Simulator;

fn compute_kernel_value(v1: ArrayView1<f64>, v2: ArrayView1<f64>) -> f64 {
    let num_qubits = v1.len();
    let mut sim1 = QuantumSimulator::new(num_qubits);
    let mut sim2 = QuantumSimulator::new(num_qubits);

    // Example encoding: apply Ry rotations with angles from v1 and v2
    for (i, &theta) in v1.iter().enumerate() {
        sim1.apply_gate(&qsim::Gate::RY { qubit: i, theta });
    }
    for (i, &theta) in v2.iter().enumerate() {
        sim2.apply_gate(&qsim::Gate::RY { qubit: i, theta });
    }

    // Compute fidelity between the two statevectors as the kernel value
    let state1 = sim1.get_statevector();
    let state2 = sim2.get_statevector();
    let fidelity = state1.fidelity(state2);
    fidelity
}

#[pyfunction]
fn quantum_kernel(x1: PyReadonlyArray1<f64>, x2: PyReadonlyArray1<f64>) -> PyResult<f64> {
    let x1 = x1.as_array();
    let x2 = x2.as_array();
    Ok(compute_kernel_value(x1, x2))
}

#[pymodule]
fn quantum_kernel_lib(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(quantum_kernel, m)?)?;
    Ok(())
}
