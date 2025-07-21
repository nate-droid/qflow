use argmin::core::{CostFunction, Error, Executor};
use argmin::solver::neldermead::NelderMead;
use qsim::QuantumSimulator; // Assuming this is the name of your simulator struct

// This struct links our problem to the `argmin` optimizer.
// It holds a mutable reference to the simulator to use it in the cost function.
struct VqaProblem<'a> {
    simulator: &'a mut QuantumSimulator,
}

/// The objective function is the core of the VQA.
/// It takes parameters from the classical optimizer and returns a "cost".
/// The optimizer's goal is to find the `params` that result in the lowest cost.
fn calculate_cost(params: &[f64], simulator: &mut QuantumSimulator) -> f64 {
    // 1. Define the Parameterized Quantum Circuit (Ansatz)
    // This is a simple circuit with two parameters.
    simulator.reset(); // Start from the |00> state
    simulator.apply_h(0);
    simulator.apply_cnot(0, 1);
    simulator.apply_ry(0, params[0]); // Apply rotation with the first parameter
    simulator.apply_rx(1, params[1]); // Apply rotation with the second parameter

    // 2. Define the Cost
    // Let's say our goal is to maximize the probability of measuring |11>.
    // The state |11> for a 2-qubit system corresponds to the 4th basis state (index 3).
    // A cost function should be minimized, so we return 1.0 - probability.
    let probability_of_11 = simulator.get_probability(3);
    1.0 - probability_of_11
}

// We must implement the `CostFunction` trait from `argmin` for our VqaProblem.
impl CostFunction for VqaProblem<'_> {
    type Param = Vec<f64>; // The parameters are a vector of floats
    type Output = f64;    // The cost is a single float

    // This `cost` method is what the optimizer will call on each iteration.
    fn cost(&self, params: &Self::Param) -> Result<Self::Output, Error> {
        let cost_value = calculate_cost(params, self.simulator);
        Ok(cost_value)
    }
}

fn main() {
    println!("🚀 Starting VQA Runner...");

    // 1. Initialize your quantum simulator for a 2-qubit system
    let mut simulator = QuantumSimulator::new(2);

    // 2. Wrap the simulator in our VQA problem struct
    let problem = VqaProblem { simulator: &mut simulator };

    // 3. Define the initial guess for the parameters
    let initial_params: Vec<f64> = vec![0.1, 0.1]; // A simple starting point

    // 4. Set up the classical optimizer
    // Nelder-Mead is a great, gradient-free optimizer to start with.
    // The vector defines the initial step size for each parameter.
    let solver = NelderMead::new(vec![vec![0.5], vec![0.5]]);

    // 5. Run the optimization!
    let result = Executor::new(problem, solver)
        .configure(|state| state.param(initial_params).max_iters(100))
        .run()
        .expect("Optimization failed to run.");

    // 6. Print the final results
    println!("\n✅ Optimization Complete!");
    println!(" -> Final Cost: {}", result.state.best_cost);
    println!(" -> Optimal Parameters: {:?}", result.state.best_param.unwrap());
}