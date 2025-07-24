use num_complex::Complex;
use rand::Rng;
use rand::distributions::{Distribution, WeightedIndex};
use serde::Serialize;
use crate::simulator::{QuantumGate};

#[derive(Serialize, Clone)]
pub struct StateVector {
    pub num_qubits: usize,
    #[serde(rename = "amplitudes")]
    pub amplitudes: Vec<Complex<f64>>,
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
