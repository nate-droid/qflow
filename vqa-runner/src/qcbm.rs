use rand::Rng;
use rand::distributions::{Distribution, WeightedIndex};
use std::cell::RefCell;
use std::collections::HashMap;

use qsim::simulator::Simulator;
use qsim::{Gate, StateVector};

const EPSILON: f64 = 1e-12;

pub trait Optimizer {
    fn update(&mut self, params: &mut [f64], grads: &[f64]);
}

pub struct AdamOptimizer {
    learning_rate: f64,
    beta1: f64,
    beta2: f64,
    epsilon: f64,
    m: Vec<f64>, // 1st moment vector (mean)
    v: Vec<f64>, // 2nd moment vector (uncentered variance)
    t: usize,    // timestep
}

impl AdamOptimizer {
    /// Creates a new AdamOptimizer.
    ///
    /// # Arguments
    /// * `num_params` - The number of parameters to optimize.
    /// * `learning_rate` - The initial learning rate (alpha).
    pub fn new(num_params: usize, learning_rate: f64) -> Self {
        Self {
            learning_rate,
            beta1: 0.92,
            beta2: 0.999,
            epsilon: 1e-8,
            m: vec![0.0; num_params],
            v: vec![0.0; num_params],
            t: 0,
        }
    }
}

impl Optimizer for AdamOptimizer {
    fn update(&mut self, params: &mut [f64], grads: &[f64]) {
        self.t += 1;
        for i in 0..params.len() {
            // Update biased moment estimates
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * grads[i];
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * grads[i].powi(2);

            // Compute bias-corrected moment estimates
            let m_hat = self.m[i] / (1.0 - self.beta1.powi(self.t as i32));
            let v_hat = self.v[i] / (1.0 - self.beta2.powi(self.t as i32));

            // Update parameters
            params[i] -= self.learning_rate * m_hat / (v_hat.sqrt() + self.epsilon);
        }
    }
}

pub struct QcbmRunner<S, F>
where
    S: Simulator,
    F: Fn(&mut S, &[f64]) + Copy,
{
    simulator: RefCell<S>,
    training_data: Vec<String>,
    ansatz: F,
    num_qubits: usize,
}

impl<S, F> QcbmRunner<S, F>
where
    S: Simulator,
    F: Fn(&mut S, &[f64]) + Copy,
{
    /// Creates a new QcbmRunner.
    pub fn new(simulator: S, ansatz: F, training_data: &[String]) -> Self {
        let num_qubits = simulator.get_num_qubits();
        QcbmRunner {
            simulator: RefCell::new(simulator),
            training_data: training_data.to_vec(),
            ansatz,
            num_qubits,
        }
    }

    /// Executes the quantum circuit and returns the full probability distribution.
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

    /// Generates samples from the model by running the circuit.
    fn get_model_samples(&self, params: &[f64], num_samples: usize) -> Vec<String> {
        let dist = self.get_model_distribution(params);
        if dist.is_empty() {
            return vec![format!("{:0width$}", 0, width = self.num_qubits); num_samples];
        }
        let mut rng = rand::thread_rng();
        let items: Vec<_> = dist.keys().cloned().collect();
        let weights: Vec<_> = dist.values().cloned().collect();
        let weighted_dist = WeightedIndex::new(&weights).unwrap();

        (0..num_samples)
            .map(|_| items[weighted_dist.sample(&mut rng)].clone())
            .collect()
    }

    /// Computes the MMD loss with a Gaussian RBF kernel.
    fn mmd_rbf_loss(target_samples: &[String], model_samples: &[String], sigma: f64) -> f64 {
        let to_vec = |s: &String| {
            s.chars()
                .map(|c| c.to_digit(10).unwrap() as f64)
                .collect::<Vec<f64>>()
        };
        let kernel = |v1: &[f64], v2: &[f64]| {
            let sq_dist: f64 = v1.iter().zip(v2.iter()).map(|(a, b)| (a - b).powi(2)).sum();
            (-sq_dist / (2.0 * sigma.powi(2))).exp()
        };

        let target_vecs: Vec<_> = target_samples.iter().map(to_vec).collect();
        let model_vecs: Vec<_> = model_samples.iter().map(to_vec).collect();

        let mut term1 = 0.0;
        for i in 0..target_vecs.len() {
            for j in 0..target_vecs.len() {
                term1 += kernel(&target_vecs[i], &target_vecs[j]);
            }
        }
        term1 /= (target_vecs.len() as f64).powi(2);

        let mut term2 = 0.0;
        for i in 0..model_vecs.len() {
            for j in 0..model_vecs.len() {
                term2 += kernel(&model_vecs[i], &model_vecs[j]);
            }
        }
        term2 /= (model_vecs.len() as f64).powi(2);

        let mut term3 = 0.0;
        for i in 0..target_vecs.len() {
            for j in 0..model_vecs.len() {
                term3 += kernel(&target_vecs[i], &model_vecs[j]);
            }
        }
        term3 /= (target_vecs.len() * model_vecs.len()) as f64;

        term1 + term2 - 2.0 * term3
    }

    /// Trains the QCBM using a provided optimizer and MMD loss with an analytical gradient.
    pub fn train<O: Optimizer>(&self, params: &mut [f64], optimizer: &mut O, epochs: usize) {
        println!("Starting training with MMD loss...");

        const NUM_MMD_SAMPLES: usize = 128;
        let mut rng = rand::thread_rng();
        let sigma = (self.num_qubits as f64).sqrt() / 2.0;
        let to_vec = |s: &String| {
            s.chars()
                .map(|c| c.to_digit(10).unwrap() as f64)
                .collect::<Vec<f64>>()
        };
        let kernel = |v1: &[f64], v2: &[f64]| {
            let sq_dist: f64 = v1.iter().zip(v2.iter()).map(|(a, b)| (a - b).powi(2)).sum();
            (-sq_dist / (2.0 * sigma.powi(2))).exp()
        };

        for epoch in 0..epochs {
            let mut gradients = vec![0.0; params.len()];

            let model_samples = self.get_model_samples(params, NUM_MMD_SAMPLES);
            let target_samples_for_epoch: Vec<String> = (0..NUM_MMD_SAMPLES)
                .map(|_| self.training_data[rng.gen_range(0..self.training_data.len())].clone())
                .collect();

            let model_vecs: Vec<_> = model_samples.iter().map(&to_vec).collect();
            let target_vecs: Vec<_> = target_samples_for_epoch.iter().map(&to_vec).collect();

            for i in 0..params.len() {
                let mut params_plus = params.to_vec();
                params_plus[i] += std::f64::consts::FRAC_PI_2;
                let dist_plus = self.get_model_distribution(&params_plus);

                let mut params_minus = params.to_vec();
                params_minus[i] -= std::f64::consts::FRAC_PI_2;
                let dist_minus = self.get_model_distribution(&params_minus);

                let mut grad_i = 0.0;
                let num_states = 1 << self.num_qubits;

                for z_idx in 0..num_states {
                    let bitstring_z = format!("{:0width$b}", z_idx, width = self.num_qubits);
                    let vec_z = to_vec(&bitstring_z);

                    let term_model: f64 = model_vecs.iter().map(|y| kernel(y, &vec_z)).sum();
                    let term_target: f64 = target_vecs.iter().map(|x| kernel(x, &vec_z)).sum();
                    let d_mmd_dp_z = 2.0
                        * (term_model / model_vecs.len() as f64
                            - term_target / target_vecs.len() as f64);

                    let p_plus_z = dist_plus.get(&bitstring_z).unwrap_or(&0.0);
                    let p_minus_z = dist_minus.get(&bitstring_z).unwrap_or(&0.0);
                    let d_p_d_theta = 0.5 * (p_plus_z - p_minus_z);

                    grad_i += d_mmd_dp_z * d_p_d_theta;
                }
                gradients[i] = grad_i;
            }

            optimizer.update(params, &gradients);

            if (epoch + 1) % 10 == 0 || epoch == epochs - 1 {
                let current_loss =
                    Self::mmd_rbf_loss(&target_samples_for_epoch, &model_samples, sigma);
                println!(
                    "Epoch {}/{} - Loss (MMD): {:.6}",
                    epoch + 1,
                    epochs,
                    current_loss
                );
            }
        }

        println!("Training finished.");
        println!("Final Parameters: {:?}", params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qsim::QuantumSimulator;

    fn simple_ry_ansatz(sim: &mut impl Simulator, params: &[f64]) {
        sim.apply_gate(&Gate::RY(0, params[0]));
    }

    fn entangling_ansatz(sim: &mut impl Simulator, params: &[f64]) {
        sim.apply_gate(&Gate::RY(0, params[0]));
        sim.apply_gate(&Gate::H(0));
        sim.apply_gate(&Gate::CX(0, 1));
    }

    #[test]
    fn test_qcbm_training_with_adam_and_mmd() {
        let target_angle = (0.75_f64).sqrt().asin() * 2.0;
        let training_data = vec![
            "1".to_string(),
            "1".to_string(),
            "1".to_string(),
            "0".to_string(),
        ];

        let sim = QuantumSimulator::new(1);
        let qcbm_runner = QcbmRunner::new(sim, simple_ry_ansatz, &training_data);
        let mut params = vec![0.1];
        let mut optimizer = AdamOptimizer::new(params.len(), 0.02);
        qcbm_runner.train(&mut params, &mut optimizer, 100);

        let final_param = params[0];
        assert!(
            (final_param.cos() - target_angle.cos()).abs() < 0.2,
            "Learned parameter is not close to target"
        );
        let final_dist = qcbm_runner.get_model_distribution(&params);
        assert!((final_dist.get("0").unwrap_or(&0.0) - 0.25).abs() < 0.1);
        assert!((final_dist.get("1").unwrap_or(&0.0) - 0.75).abs() < 0.1);
    }

    #[test]
    fn test_qcbm_learns_entangled_state_with_adam_and_mmd() {
        let training_data = vec![
            "00".to_string(),
            "11".to_string(),
            "00".to_string(),
            "11".to_string(),
        ];

        let sim = QuantumSimulator::new(2);
        let qcbm_runner = QcbmRunner::new(sim, entangling_ansatz, &training_data);
        let mut params = vec![0.2];
        let mut optimizer = AdamOptimizer::new(params.len(), 0.01);
        qcbm_runner.train(&mut params, &mut optimizer, 100);

        assert!(
            params[0].cos().abs() > 0.95,
            "Parameter should converge to ~0"
        );
        let final_dist = qcbm_runner.get_model_distribution(&params);
        let p00 = final_dist.get("00").unwrap_or(&0.0);
        let p11 = final_dist.get("11").unwrap_or(&0.0);
        let p01 = final_dist.get("01").unwrap_or(&0.0);
        let p10 = final_dist.get("10").unwrap_or(&0.0);

        assert!((p00 - 0.5).abs() < 0.1, "P('00') should be ~0.5");
        assert!((p11 - 0.5).abs() < 0.1, "P('11') should be ~0.5");
        assert!(*p01 < 0.1, "P('01') should be ~0");
        assert!(*p10 < 0.1, "P('10') should be ~0");
    }
}
