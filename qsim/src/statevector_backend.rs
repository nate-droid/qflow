// src/simulator/statevector_backend.rs
use crate::{StateVector};
use crate::parser::Gate;
use crate::api::{SimulatorApi, SimError, Pauli};
use num_complex::Complex;
use rand::thread_rng;
use std::collections::HashMap;
use std::f64::consts::FRAC_1_SQRT_2;
use crate::circuit::Circuit;

pub struct StatevectorSimulator {
    num_qubits: usize,
    state: StateVector,
}

impl StatevectorSimulator {
    pub fn new(num_qubits: usize) -> Self {
        Self { num_qubits, state: StateVector::new(num_qubits) }
    }

    fn apply_gate(&mut self, g: &Gate) {
        // Constants
        let h = [
            [Complex::new(FRAC_1_SQRT_2, 0.0), Complex::new(FRAC_1_SQRT_2, 0.0)],
            [Complex::new(FRAC_1_SQRT_2, 0.0), Complex::new(-FRAC_1_SQRT_2, 0.0)],
        ];
        let x = [
            [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        ];
        let y = [
            [Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
            [Complex::new(0.0, 1.0), Complex::new(0.0, 0.0)],
        ];
        let z = [
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            [Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
        ];

        match *g {
            Gate::I { qubit } => {
                // no-op (skip)
                let _ = qubit;
            }
            Gate::H { qubit } => self.state.apply_single_qubit_gate(&h, qubit),
            Gate::X { qubit } => self.state.apply_single_qubit_gate(&x, qubit),
            Gate::Y { qubit } => self.state.apply_single_qubit_gate(&y, qubit),
            Gate::Z { qubit } => self.state.apply_single_qubit_gate(&z, qubit),

            Gate::RX { qubit, theta } => {
                // Rx(θ) = cos(θ/2) I - i sin(θ/2) X
                let c = theta * 0.5;
                let (ct, st) = (c.cos(), c.sin());
                let m = [
                    [Complex::new(ct, 0.0), Complex::new(0.0, -st)],
                    [Complex::new(0.0, -st), Complex::new(ct, 0.0)],
                ];
                self.state.apply_single_qubit_gate(&m, qubit)
            }
            Gate::RY { qubit, theta } => {
                // Ry(θ) = cos(θ/2) I - i sin(θ/2) Y  -> matrix is real
                let c = theta * 0.5;
                let (ct, st) = (c.cos(), c.sin());
                let m = [
                    [Complex::new(ct, 0.0), Complex::new(-st, 0.0)],
                    [Complex::new(st, 0.0), Complex::new(ct, 0.0)],
                ];
                self.state.apply_single_qubit_gate(&m, qubit)
            }
            Gate::RZ { qubit, theta } => {
                // Rz(θ) = diag(e^{-iθ/2}, e^{+iθ/2})
                let c = theta * 0.5;
                let (ct, st) = (c.cos(), c.sin());
                let m = [
                    [Complex::new(ct, -st), Complex::new(0.0, 0.0)],
                    [Complex::new(0.0, 0.0), Complex::new(ct, st)],
                ];
                self.state.apply_single_qubit_gate(&m, qubit)
            }

            Gate::CX { control, target } | Gate::CNOT { control, target } => {
                self.state.apply_cx(control, target)
            }

            // If you have a `Measure` gate in parsed circuits, you can ignore it here
            // (tests call measure() explicitly), or do a full-measure collapse:
            Gate::Measure => {
                let _ = self.state.measure_all(&mut thread_rng());
            }
        }
    }

    fn apply_circuit(&mut self, c: &Circuit) {
        for moment in &c.moments {
            for g in moment {
                self.apply_gate(g);
            }
        }
    }
}

impl SimulatorApi for StatevectorSimulator {
    fn reset(&mut self, n: usize) {
        self.num_qubits = n;
        self.state = StateVector::new(n);
    }

    fn run(&mut self, circuit: &Circuit) -> Result<(), SimError> {
        if self.num_qubits != circuit.num_qubits {
            self.reset(circuit.num_qubits);
        } else {
            self.state.reset();
        }
        self.apply_circuit(circuit);
        Ok(())
    }

    fn statevector(&self) -> &StateVector { &self.state }

    fn measure(&mut self, qubit: usize) -> Result<u8, SimError> {
        if qubit >= self.num_qubits { return Err(SimError::Qubit(qubit)); }
        // Prefer the single-qubit collapse if you added it; otherwise use measure_all and extract the bit.
        #[allow(unused_mut)]
        let mut outcome = None;

        // If you implemented `measure_qubit_in_z` on StateVector:
        #[allow(unused_variables)]
        {
            // comment out if you didn't add it
            // outcome = Some(self.state.measure_qubit_in_z(qubit, &mut thread_rng()));
        }

        let m = outcome.unwrap_or_else(|| {
            let idx = self.state.measure_all(&mut thread_rng());
            ((idx >> qubit) & 1) as u8
        });
        Ok(m)
    }

    fn expectation(&self, ops: &[(Pauli, usize)]) -> Result<f64, SimError> {
        // If you implemented `expectation_pauli_string` on StateVector:
        #[allow(unreachable_code)]
        {
            // comment out if you didn't add it
            // return Ok(self.state.expectation_pauli_string(ops));
        }
        // Generic fallback: apply P|ψ⟩ on a clone and compute <ψ|φ>
        let mut phi = self.state.clone();
        let i = Complex::new(0.0, 1.0);

        let id = [
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
        ];
        let px = [
            [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        ];
        let py = [
            [Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
            [Complex::new(0.0, 1.0), Complex::new(0.0, 0.0)],
        ];
        let pz = [
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            [Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
        ];

        for &(p, q) in ops {
            match p {
                Pauli::I => phi.apply_single_qubit_gate(&id, q),
                Pauli::X => phi.apply_single_qubit_gate(&px, q),
                Pauli::Y => phi.apply_single_qubit_gate(&py, q),
                Pauli::Z => phi.apply_single_qubit_gate(&pz, q),
            }
        }

        let mut acc = Complex::new(0.0, 0.0);
        for (a, b) in self.state.amplitudes.iter().zip(phi.amplitudes.iter()) {
            acc += a.conj() * b;
        }
        Ok(acc.re)
    }

    fn sample(&self, shots: u32) -> Result<HashMap<String, u32>, SimError> {
        // If you implemented `sample_counts` on StateVector:
        #[allow(unreachable_code)]
        {
            // comment out if you didn't add it
            // return Ok(self.state.sample_counts(shots));
        }

        use rand::distributions::{Distribution, WeightedIndex};
        let probs: Vec<f64> = self.state.amplitudes.iter().map(|a| a.norm_sqr()).collect();
        let dist = WeightedIndex::new(&probs).map_err(|e| SimError::Internal(e.to_string()))?;

        let mut rng = thread_rng();
        let mut counts = HashMap::new();
        let width = self.num_qubits;
        for _ in 0..shots {
            let idx = dist.sample(&mut rng);
            let bitstr = format!("{:0width$b}", idx, width = width);
            *counts.entry(bitstr).or_insert(0) += 1;
        }
        Ok(counts)
    }
}
