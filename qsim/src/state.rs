use num_complex::Complex;
use rand::Rng;
use rand::distributions::{Distribution, WeightedIndex};
use serde::Serialize;

/// Represents the state of an n-qubit system.
/// The state is stored as a vector of 2^n complex amplitudes.
#[derive(Serialize, Clone)]
pub struct StateVector {
    pub num_qubits: usize,
    #[serde(rename = "amplitudes")]
    pub amplitudes: Vec<Complex<f64>>,
}

impl StateVector {
    /// Creates a new state vector for `num_qubits` qubits,
    /// initialized to the |0...0> state.
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

    /// Applies a single-qubit gate to the specified qubit.
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

    /// Applies a Controlled-X (CNOT) gate.
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

    /// Simulates measuring all qubits, collapsing the state vector.
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

        // Apply X to qubit 1 to get |10> (state 2)
        state.apply_single_qubit_gate(&pauli_x, 1);

        let mut rng = thread_rng();
        let result = state.measure_all(&mut rng);

        assert_eq!(result, 2);
        assert!(approx_eq(state.amplitudes[2], Complex::new(1.0, 0.0)));
    }
}
