use std::collections::HashMap;
use std::cell::RefCell;
use num_complex::Complex;
use qsim::simulator::Simulator;

// A small constant to prevent division by zero or log(0) in the loss function.
const EPSILON: f64 = 1e-12;

/// Represents a Quantum Circuit Born Machine runner.
/// It's designed to train a parameterized quantum circuit (ansatz)
/// to replicate a target probability distribution from classical data.
pub struct QcbmRunner<S, F>
where
    S: Simulator,
    F: Fn(&mut S, &[f64]) + Copy,
{
    simulator: RefCell<S>,
    target_distribution: HashMap<String, f64>,
    ansatz: F,
    num_qubits: usize,
}

impl<S, F> QcbmRunner<S, F>
where
    S: Simulator,
    F: Fn(&mut S, &[f64]) + Copy,
{
    /// Creates a new QcbmRunner.
    ///
    /// # Arguments
    /// * `simulator` - An instance of your quantum simulator.
    /// * `ansatz` - A closure representing the parameterized quantum circuit.
    /// * `training_data` - A slice of bitstrings (e.g., "011", "101") representing the dataset to learn.
    pub fn new(simulator: S, ansatz: F, training_data: &[String]) -> Self {
        let num_qubits = simulator.get_num_qubits();
        let target_distribution = Self::calculate_target_distribution(training_data, num_qubits);

        QcbmRunner {
            simulator: RefCell::new(simulator),
            target_distribution,
            ansatz,
            num_qubits,
        }
    }

    /// Calculates the probability distribution from the input data.
    fn calculate_target_distribution(data: &[String], num_qubits: usize) -> HashMap<String, f64> {
        let mut counts = HashMap::new();
        for item in data {
            // Ensure data has the correct number of bits
            if item.len() == num_qubits {
                *counts.entry(item.clone()).or_insert(0) += 1;
            }
        }

        let total_samples = data.len() as f64;
        if total_samples == 0.0 {
            return HashMap::new();
        }

        counts
            .into_iter()
            .map(|(key, count)| (key, count as f64 / total_samples))
            .collect()
    }

    /// Executes the quantum circuit with the given parameters and returns the resulting
    /// probability distribution by calculating it from the final statevector.
    pub fn get_model_distribution(&self, params: &[f64]) -> HashMap<String, f64> {
        let mut sim = self.simulator.borrow_mut();
        sim.reset();
        (self.ansatz)(&mut sim, params);

        let statevector = sim.get_statevector();
        let mut distribution = HashMap::new();

        for i in 0..statevector.len() {
            let probability = statevector[i].norm_sqr();
            if probability > EPSILON {
                let bitstring = format!("{:0width$b}", i, width = self.num_qubits);
                distribution.insert(bitstring, probability);
            }
        }
        distribution
    }

    /// Calculates the Kullback-Leibler (KL) divergence between the target and model distributions.
    /// KL(P || Q) = Σ P(x) * log(P(x) / Q(x))
    /// This serves as the loss function for our training.
    fn kl_divergence(
        target_dist: &HashMap<String, f64>,
        model_dist: &HashMap<String, f64>,
    ) -> f64 {
        target_dist
            .iter()
            .map(|(key, p_prob)| {
                let q_prob = model_dist.get(key).unwrap_or(&EPSILON);
                p_prob * (p_prob.ln() - q_prob.ln())
            })
            .sum()
    }

    fn l2_distance(
        target_dist: &HashMap<String, f64>,
        model_dist: &HashMap<String, f64>,
        num_qubits: usize,
    ) -> f64 {
        let mut total_error = 0.0;
        let num_states = 1 << num_qubits;

        // Iterate over all possible bitstrings in the Hilbert space
        for i in 0..num_states {
            let bitstring = format!("{:0width$b}", i, width = num_qubits);
            let p_prob = target_dist.get(&bitstring).unwrap_or(&0.0);
            let q_prob = model_dist.get(&bitstring).unwrap_or(&0.0);
            total_error += (p_prob - q_prob).powi(2);
        }
        total_error
    }

    /// Trains the QCBM using gradient descent.
    ///
    /// # Arguments
    /// * `params` - The starting parameters for the ansatz, which will be updated in place.
    /// * `learning_rate` - The step size for the optimizer.
    /// * `epochs` - The number of training iterations.
    pub fn train(&self, params: &mut [f64], learning_rate: f64, epochs: usize) {
        println!("Target Distribution: {:?}", self.target_distribution);
        println!("Starting training with {} parameters...", params.len());

        for epoch in 0..epochs {
            let mut gradients = vec![0.0; params.len()];
            let current_dist = self.get_model_distribution(params);

            // Calculate gradient for each parameter
            for i in 0..params.len() {
                // To get the full gradient of the loss function, we use the chain rule:
                // d(Loss)/dθ = Σ_x [ d(Loss)/dP(x) * dP(x)/dθ ]
                // d(Loss)/dP(x) = -2 * (P_target(x) - P_model(x))
                // dP(x)/dθ is found with the parameter-shift rule.

                // Shift parameter up to get P(x | θ + π/2)
                let mut params_plus = params.to_vec();
                params_plus[i] += std::f64::consts::FRAC_PI_2;
                let dist_plus = self.get_model_distribution(&params_plus);

                // Shift parameter down to get P(x | θ - π/2)
                let mut params_minus = params.to_vec();
                params_minus[i] -= std::f64::consts::FRAC_PI_2;
                let dist_minus = self.get_model_distribution(&params_minus);

                let mut grad_i = 0.0;
                let num_states = 1 << self.num_qubits;
                // Sum over all possible states 'x'
                for j in 0..num_states {
                    let bitstring = format!("{:0width$b}", j, width = self.num_qubits);

                    let p_target = self.target_distribution.get(&bitstring).unwrap_or(&0.0);
                    let p_model = current_dist.get(&bitstring).unwrap_or(&0.0);

                    // d(Loss)/dP(x)
                    let loss_grad_p = -2.0 * (p_target - p_model);

                    // dP(x)/dθ using parameter-shift
                    let p_plus = dist_plus.get(&bitstring).unwrap_or(&0.0);
                    let p_minus = dist_minus.get(&bitstring).unwrap_or(&0.0);
                    let p_grad_theta = 0.5 * (p_plus - p_minus);

                    grad_i += loss_grad_p * p_grad_theta;
                }
                gradients[i] = grad_i;
            }

            // Update parameters using gradient descent
            for i in 0..params.len() {
                params[i] -= learning_rate * gradients[i];
            }

            // Print progress
            if (epoch + 1) % 10 == 0 || epoch == epochs - 1 {
                let current_dist = self.get_model_distribution(params);
                let current_loss = Self::l2_distance(&self.target_distribution, &current_dist, self.num_qubits);
                println!("Epoch {}/{} - Loss (L2 Distance): {:.6}", epoch + 1, epochs, current_loss);
            }
        }

        println!("Training finished.");
        println!("Final Parameters: {:?}", params);
    }
}


#[cfg(test)]
mod tests {
    use qsim::{Gate, StateVector};
    use super::*;

    /// A very basic mock simulator for testing purposes.
    /// It doesn't actually simulate quantum mechanics but allows us to
    /// track gate applications and set a statevector manually for testing.
    struct MockSimulator {
        num_qubits: usize,
        statevector: StateVector,
        // We can track applied gates to see if the ansatz is called correctly
        gate_log: Vec<Gate>,
    }

    impl MockSimulator {
        fn new(num_qubits: usize) -> Self {
            let dim = 1 << num_qubits;
            let mut statevector = vec![Complex::new(0.0, 0.0); dim];
            statevector[0] = Complex::new(1.0, 0.0); // Start in |0...0>
            Self {
                num_qubits,
                gate_log: Vec::new(),
                statevector: StateVector::from(statevector),
            }
        }
    }

    // A simple implementation of the Simulator trait for our mock object.
    impl Simulator for MockSimulator {
        fn reset(&mut self) {
            let dim = 1 << self.num_qubits;
            self.statevector = vec![Complex::new(0.0, 0.0); dim].into();
            self.statevector = {
                let mut sv = vec![Complex::new(0.0, 0.0); dim];
                sv[0] = Complex::new(1.0, 0.0);
                StateVector::from(sv)
            };
            self.gate_log.clear();
        }

        fn apply_gate(&mut self, gate: &Gate) {
            // In a real simulator, this would apply the gate matrix to the statevector.
            // For the mock, we just log the gate to know it was called.
            self.gate_log.push(gate.clone());

            // A toy implementation for RY to make the test meaningful
            if let Gate::RY(.., angle) = gate {
                let c = (angle / 2.0).cos();
                let s = (angle / 2.0).sin();
                let old_zero = self.statevector[0];
                let old_one = self.statevector[1];
                if let Some(state) = self.statevector.as_mut_slice().get_mut(0..2) {
                    state[0] = old_zero * c - old_one * s;
                    state[1] = old_zero * s + old_one * c;
                }
            }
        }

        fn get_statevector(&self) -> &StateVector {
            &self.statevector
        }

        fn get_num_qubits(&self) -> usize {
            self.num_qubits
        }

        fn measure_pauli_string_expectation(&mut self, _operators: Vec<Gate>) -> f64 {
            // Not needed for QCBM, return a dummy value.
            0.0
        }
    }

    /// A simple ansatz for testing: a single RY rotation on the first qubit.
    fn simple_ry_ansatz(sim: &mut impl Simulator, params: &[f64]) {
        sim.apply_gate(&Gate::RY(0, params[0]));
    }

    #[test]
    fn test_qcbm_training() {
        let target_angle = (0.75_f64).sqrt().asin() * 2.0;

        let training_data = vec![
            "1".to_string(), "1".to_string(), "1".to_string(), "0".to_string()
        ];

        // 2. Initialization
        let mock_sim = MockSimulator::new(1);
        let qcbm_runner = QcbmRunner::new(mock_sim, simple_ry_ansatz, &training_data);

        // Start with a small, non-zero parameter to avoid getting stuck at a saddle point.
        let mut params = vec![0.1];

        // 3. Train the model
        qcbm_runner.train(&mut params, 0.4, 50);

        // 4. Assertions
        let final_param = params[0];
        println!("Target RY angle: {:.4}, Found angle: {:.4}", target_angle, final_param);

        // Check if the learned parameter is close to the ideal one.
        // We check the cosine to handle periodicity (e.g. theta vs -theta or theta + 2pi).
        assert!((final_param.cos() - target_angle.cos()).abs() < 0.1, "Learned parameter is not close to target");

        // Check if the final distribution is close to the target distribution
        let final_dist = qcbm_runner.get_model_distribution(&params);
        let p0 = final_dist.get("0").unwrap_or(&0.0);
        let p1 = final_dist.get("1").unwrap_or(&0.0);

        assert!((p0 - 0.25).abs() < 0.05, "Probability of '0' should be close to 0.25");
        assert!((p1 - 0.75).abs() < 0.05, "Probability of '1' should be close to 0.75");
    }
}
