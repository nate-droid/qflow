use crate::api::Pauli;
use crate::simulator::QuantumGate;
use num_complex::Complex;
use rand::Rng;
use rand::distributions::{Distribution, WeightedIndex};
use serde::Serialize;
use std::collections::HashMap;
use std::ops::Deref;

#[derive(Serialize, Clone, Debug)]
pub struct StateVector {
    pub num_qubits: usize,
    #[serde(rename = "amplitudes")]
    pub amplitudes: Vec<Complex<f64>>,
}

impl StateVector {
    pub fn as_mut_slice(&mut self) -> &mut [Complex<f64>] {
        self.amplitudes.as_mut_slice()
    }

    pub fn measure_qubit_in_z<R: Rng + ?Sized>(&mut self, qubit: usize, rng: &mut R) -> u8 {
        assert!(qubit < self.num_qubits, "qubit out of range");

        let n = self.num_qubits;
        let stride = 1usize << qubit;

        // P(1) = sum |amp[i]|^2 over basis states where bit 'qubit' == 1
        let mut p1 = 0.0f64;
        for i in 0..self.amplitudes.len() {
            if (i & stride) != 0 {
                p1 += self.amplitudes[i].norm_sqr();
            }
        }

        // Sample outcome
        let r: f64 = rng.r#gen();
        let outcome = if r < p1 { 1u8 } else { 0u8 };

        // Collapse amplitudes inconsistent with outcome and renormalize
        let p_keep = if outcome == 1 { p1 } else { 1.0 - p1 };
        let norm = if p_keep > 0.0 { p_keep.sqrt() } else { 1.0 };

        for i in 0..self.amplitudes.len() {
            let bit = ((i & stride) != 0) as u8;
            if bit != outcome {
                self.amplitudes[i] = Complex::new(0.0, 0.0);
            } else if norm != 0.0 {
                self.amplitudes[i] /= norm;
            }
        }

        outcome
    }

    /// ⟨ψ|P|ψ⟩ for a Pauli string, non-destructive.
    pub fn expectation_pauli_string(&self, ops: &[(Pauli, usize)]) -> f64 {
        // Build |φ⟩ = P|ψ⟩ by applying each single-qubit Pauli to a clone
        let mut phi = self.clone();

        let i = Complex::new(0.0, 1.0);
        let id = [
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
        ];
        let x = [
            [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        ];
        let y = [[Complex::new(0.0, 0.0), -i], [i, Complex::new(0.0, 0.0)]];
        let z = [
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            [Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
        ];

        for &(p, q) in ops {
            match p {
                Pauli::I => phi.apply_single_qubit_gate(&id, q),
                Pauli::X => phi.apply_single_qubit_gate(&x, q),
                Pauli::Y => phi.apply_single_qubit_gate(&y, q),
                Pauli::Z => phi.apply_single_qubit_gate(&z, q),
            }
        }

        // ⟨ψ|φ⟩
        let mut acc = Complex::new(0.0, 0.0);
        for (a, b) in self.amplitudes.iter().zip(phi.amplitudes.iter()) {
            acc += a.conj() * b;
        }
        // Expectation for Hermitian Pauli strings should be real; return Re just in case
        acc.re
    }

    /// Sample computational-basis outcomes `shots` times and return counts.
    pub fn sample_counts(&self, shots: u32) -> HashMap<String, u32> {
        let probs: Vec<f64> = self.amplitudes.iter().map(|a| a.norm_sqr()).collect();
        // WeightedIndex expects nonnegative and (usually) sums to ~1
        let dist = WeightedIndex::new(&probs).expect("invalid probability distribution");

        let mut rng = rand::thread_rng();
        let mut counts: HashMap<String, u32> = HashMap::new();
        let width = self.num_qubits;

        for _ in 0..shots {
            let idx = dist.sample(&mut rng);
            // bitstring with q_{width-1} ... q_0 (MSB..LSB)
            let bitstr = format!("{:0width$b}", idx, width = width);
            *counts.entry(bitstr).or_insert(0) += 1;
        }
        counts
    }
}

impl StateVector {
    pub fn new(num_qubits: usize) -> Self {
        let size = 1 << num_qubits; // 2^num_qubits
        let mut amplitudes = vec![Complex::new(0.0, 0.0); size];
        if !amplitudes.is_empty() {
            amplitudes[0] = Complex::new(1.0, 0.0);
        }
        Self {
            num_qubits,
            amplitudes,
        }
    }

    pub fn apply_single_qubit_gate(
        &mut self,
        gate_matrix: &[[Complex<f64>; 2]; 2],
        target_qubit: usize,
    ) {
        let mut new_amplitudes = self.amplitudes.clone();
        let k = 1 << target_qubit;

        for i in 0..self.amplitudes.len() {
            if (i & k) == 0 {
                let j = i | k;
                let amp_i = self.amplitudes[i];
                let amp_j = self.amplitudes[j];

                new_amplitudes[i] = gate_matrix[0][0] * amp_i + gate_matrix[0][1] * amp_j;
                new_amplitudes[j] = gate_matrix[1][0] * amp_i + gate_matrix[1][1] * amp_j;
            }
        }
        self.amplitudes = new_amplitudes;
    }

    pub fn apply_multi_qubit_gate(
        &mut self,
        gate_matrix: &[[Complex<f64>; 2]; 2],
        target_qubits: &[usize],
    ) {
        let n = target_qubits.len();
        let mut new_amplitudes = self.amplitudes.clone();

        for i in 0..self.amplitudes.len() {
            // Find the basis state indices for the subspace spanned by the target qubits
            let mut basis_indices = Vec::with_capacity(1 << n);
            for b in 0..(1 << n) {
                let mut idx = i;
                for (bit_pos, &qubit) in target_qubits.iter().enumerate() {
                    let bit = (b >> bit_pos) & 1;
                    if bit == 1 {
                        idx |= 1 << qubit;
                    } else {
                        idx &= !(1 << qubit);
                    }
                }
                basis_indices.push(idx);
            }
            // Only update amplitudes for the "lowest" representative in each subspace
            if basis_indices[0] == i {
                let mut amps = vec![Complex::new(0.0, 0.0); 1 << n];
                for (j, &idx) in basis_indices.iter().enumerate() {
                    amps[j] = self.amplitudes[idx];
                }
                // Apply the gate matrix (assumed to be 2^n x 2^n)
                let gate_size = 1 << n;
                let gate: &[[Complex<f64>; 2]] = gate_matrix as &[_];
                let mut new_amps = vec![Complex::new(0.0, 0.0); gate_size];
                for row in 0..gate_size {
                    for col in 0..gate_size {
                        new_amps[row] += gate[row][col] * amps[col];
                    }
                }
                for (j, &idx) in basis_indices.iter().enumerate() {
                    new_amplitudes[idx] = new_amps[j];
                }
            }
        }
        self.amplitudes = new_amplitudes;
    }

    pub fn apply_cx(&mut self, control_qubit: usize, target_qubit: usize) {
        let mut new_amplitudes = self.amplitudes.clone();
        let control_mask = 1 << control_qubit;
        let target_mask = 1 << target_qubit;

        for i in 0..self.amplitudes.len() {
            if (i & control_mask) != 0 && (i & target_mask) == 0 {
                let j = i | target_mask;
                new_amplitudes.swap(i, j);
            }
        }
        self.amplitudes = new_amplitudes;
    }

    pub fn measure_all(&mut self, rng: &mut impl Rng) -> usize {
        let probabilities: Vec<f64> = self.amplitudes.iter().map(|a| a.norm_sqr()).collect();
        let dist =
            WeightedIndex::new(&probabilities).expect("Failed to create weighted distribution.");
        let measured_index = dist.sample(rng);

        for (i, amp) in self.amplitudes.iter_mut().enumerate() {
            *amp = if i == measured_index {
                Complex::new(1.0, 0.0)
            } else {
                Complex::new(0.0, 0.0)
            };
        }
        measured_index
    }

    pub fn reset(&mut self) {
        for amp in &mut self.amplitudes {
            *amp = Complex::new(0.0, 0.0);
        }
        if !self.amplitudes.is_empty() {
            self.amplitudes[0] = Complex::new(1.0, 0.0);
        }
    }

    // fidelity = |⟨ψ|φ⟩|²
    pub fn fidelity(&self, other: &StateVector) -> f64 {
        assert_eq!(
            self.amplitudes.len(),
            other.amplitudes.len(),
            "StateVectors must have the same dimension"
        );
        let inner_product: Complex<f64> = self
            .amplitudes
            .iter()
            .zip(&other.amplitudes)
            .map(|(a, b)| a.conj() * b)
            .sum();
        inner_product.norm_sqr()
    }
}

impl From<Vec<Complex<f64>>> for StateVector {
    fn from(vec: Vec<Complex<f64>>) -> Self {
        StateVector {
            num_qubits: 0,
            amplitudes: vec,
        }
    }
}

impl Deref for StateVector {
    type Target = [Complex<f64>];
    fn deref(&self) -> &Self::Target {
        &self.amplitudes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    const EPSILON: f64 = 1e-9;

    fn approx_eq(a: Complex<f64>, b: Complex<f64>) -> bool {
        (a.re - b.re).abs() < EPSILON && (a.im - b.im).abs() < EPSILON
    }

    #[test]
    fn test_state_vector_initialization() {
        let num_qubits = 3;
        let state = StateVector::new(num_qubits);
        assert_eq!(state.num_qubits, num_qubits);
        assert_eq!(state.amplitudes.len(), 1 << num_qubits);
        assert!(approx_eq(state.amplitudes[0], Complex::new(1.0, 0.0)));
        for i in 1..state.amplitudes.len() {
            assert!(approx_eq(state.amplitudes[i], Complex::new(0.0, 0.0)));
        }
    }

    #[test]
    fn test_measurement() {
        let pauli_x = [
            [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        ];
        let mut state = StateVector::new(2); // State is |00>

        state.apply_single_qubit_gate(&pauli_x, 1);

        let mut rng = thread_rng();
        let result = state.measure_all(&mut rng);

        assert_eq!(result, 2);
        assert!(approx_eq(state.amplitudes[2], Complex::new(1.0, 0.0)));
    }
}
