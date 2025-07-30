use crate::parser::{Declaration, Gate as SymbolicGate, Value};
use qsim::Gate as ConcreteGate; // Your existing, concrete Gate enum from qsim
use std::collections::HashMap;

// ================================================================================================
// |                                    Workflow State & Definitions                               |
// ================================================================================================

/// A circuit definition, stored in the workflow's state after being parsed.
/// It keeps the circuit's name, qubit count, and the symbolic representation of its gates.
#[derive(Debug, Clone)]
pub struct CircuitDef {
    pub name: String,
    pub qubits: u64,
    pub body: Vec<SymbolicGate>,
}

/// The main execution engine. It holds the state of a QCL script,
/// including all defined parameters and circuits.
pub struct Workflow {
    /// Stores the concrete f64 values of parameters, e.g., 'theta_1' -> 0.785
    pub params: HashMap<String, f64>,
    /// Stores the definitions of circuits, e.g., 'vqe_ansatz' -> CircuitDef { ... }
    pub circuits: HashMap<String, CircuitDef>,
    // In the future, you could add hashmaps for observables, optimizers, etc.
}

// ================================================================================================
// |                                     Execution Logic                                          |
// ================================================================================================

impl Workflow {
    /// Creates a new, empty workflow.
    pub fn new() -> Self {
        Workflow {
            params: HashMap::new(),
            circuits: HashMap::new(),
        }
    }

    /// The main entry point for the interpreter. It consumes the AST from the parser
    /// and populates the workflow's state. When it encounters a `(run ...)` command,
    /// it triggers the simulation.
    pub fn execute(&mut self, declarations: Vec<Declaration>) -> Result<(), String> {
        for decl in declarations {
            match decl {
                Declaration::DefParam { name, value } => {
                    println!("[Workflow] Defining parameter: '{}' = {}", name, value);
                    self.params.insert(name, value);
                }
                Declaration::DefCircuit { name, qubits, body } => {
                    println!("[Workflow] Defining circuit: '{}'", name);
                    let circuit_def = CircuitDef { name: name.clone(), qubits, body };
                    self.circuits.insert(name, circuit_def);
                }
                Declaration::DefObs { name, operator } => {
                    println!("[Workflow] Defining observable: '{}' = {}", name, operator);
                    // In a real implementation, you would parse the operator string
                    // and store it in a dedicated `observables` hashmap.
                }
                Declaration::Run(run_args) => {
                    println!("[Workflow] --- Triggering Run ---");
                    // The `(run ...)` command triggers the actual simulation.
                    self.run_simulation(&run_args)?;
                }
            }
        }
        Ok(())
    }

    /// This function orchestrates the simulation based on the `(run ...)` arguments.
    fn run_simulation(&self, args: &HashMap<String, Value>) -> Result<(), String> {
        // Find the circuit name from the run arguments.
        let circuit_name = match args.get("circuit") {
            Some(Value::Symbol(s)) => s,
            _ => return Err("Run command must specify a circuit, e.g., (run (circuit: 'my_circ'))".to_string()),
        };

        let circuit_def = self.circuits.get(circuit_name)
            .ok_or_else(|| format!("Circuit '{}' not found for run command", circuit_name))?;

        println!("[Workflow] Building concrete circuit for '{}'", circuit_def.name);

        // This is the bridge between the symbolic language and the concrete simulator.
        let concrete_circuit = self.build_concrete_circuit(circuit_def)?;

        println!("[Workflow] Concrete circuit built with {} gates.", concrete_circuit.len());
        // Here, you would pass the `concrete_circuit` to your `qsim` simulator.
        // let mut simulator = qsim::Simulator::new(circuit_def.qubits);
        // simulator.run(&concrete_circuit);
        // let expectation = simulator.measure_expectation(...);

        Ok(())
    }

    /// Converts a symbolic circuit definition into a list of concrete, runnable gates
    /// from your `qsim` crate by resolving all parameters.
    fn build_concrete_circuit(&self, circuit_def: &CircuitDef) -> Result<Vec<ConcreteGate>, String> {
        circuit_def.body.iter().map(|gate| self.build_concrete_gate(gate)).collect()
    }

    /// The core translation logic. It converts a single symbolic gate into a concrete one.
    fn build_concrete_gate(&self, symbolic_gate: &SymbolicGate) -> Result<ConcreteGate, String> {
        // Helper to extract a qubit index from the arguments.
        let get_qubit = |arg_idx: usize| -> Result<usize, String> {
            match &symbolic_gate.args.get(arg_idx) {
                Some(Value::Num(n)) => Ok(*n as usize),
                _ => Err(format!("Expected a qubit index (number) for gate '{}'", symbolic_gate.name)),
            }
        };

        // Helper to resolve a rotation angle, which can be a number or a symbol.
        let get_angle = |arg_idx: usize| -> Result<f64, String> {
            match &symbolic_gate.args.get(arg_idx) {
                Some(Value::Num(n)) => Ok(*n),
                Some(Value::Symbol(s)) => self.params.get(s)
                    .cloned()
                    .ok_or_else(|| format!("Undefined parameter '{}' for gate '{}'", s, symbolic_gate.name)),
                _ => Err(format!("Invalid argument for angle in gate '{}'", symbolic_gate.name)),
            }
        };

        // Match on the gate name and construct the appropriate `qsim::Gate` enum variant.
        match symbolic_gate.name.as_str() {
            "H" => Ok(ConcreteGate::H { qubit: get_qubit(0)? }),
            "X" => Ok(ConcreteGate::X { qubit: get_qubit(0)? }),
            "CX" | "CNOT" => Ok(ConcreteGate::CX { control: get_qubit(0)?, target: get_qubit(1)? }),
            "RY" => Ok(ConcreteGate::RY { qubit: get_qubit(1)?, theta: get_angle(0)? }),
            "RZ" => Ok(ConcreteGate::RZ { qubit: get_qubit(1)?, theta: get_angle(0)? }),
            // ... add other gates from your qsim::Gate enum here ...
            _ => Err(format!("Unknown gate name '{}'", symbolic_gate.name)),
        }
    }
}


// ================================================================================================
// |                                             Tests                                            |
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the `execute` method correctly populates the workflow's state.
    #[test]
    fn test_workflow_execution_and_state() {
        let declarations = vec![
            Declaration::DefParam {
                name: "theta".to_string(),
                value: 1.57,
            },
            Declaration::DefCircuit {
                name: "my_circ".to_string(),
                qubits: 1,
                body: vec![SymbolicGate {
                    name: "RX".to_string(),
                    args: vec![Value::Symbol("theta".to_string()), Value::Num(0.0)],
                }],
            },
        ];

        let mut workflow = Workflow::new();
        let result = workflow.execute(declarations);

        assert!(result.is_ok());
        assert_eq!(workflow.params.get("theta"), Some(&1.57));
        assert!(workflow.circuits.contains_key("my_circ"));
        assert_eq!(workflow.circuits.get("my_circ").unwrap().qubits, 1);
    }

    /// Test the successful conversion of a symbolic circuit to a concrete one.
    #[test]
    fn test_concrete_circuit_building() {
        let mut workflow = Workflow::new();
        workflow.params.insert("angle".to_string(), 3.14);

        let circuit_def = CircuitDef {
            name: "test_circ".to_string(),
            qubits: 2,
            body: vec![
                SymbolicGate { name: "H".to_string(), args: vec![Value::Num(0.0)] },
                SymbolicGate { name: "RY".to_string(), args: vec![Value::Symbol("angle".to_string()), Value::Num(1.0)] },
            ],
        };

        let concrete_circuit = workflow.build_concrete_circuit(&circuit_def).unwrap();

        assert_eq!(concrete_circuit.len(), 2);
        // Assuming your qsim::Gate is PartialEq
        assert_eq!(concrete_circuit[0], ConcreteGate::H { qubit: 0 });
        assert_eq!(concrete_circuit[1], ConcreteGate::RY { qubit: 1, theta: 3.14 });
    }

    /// Test that building a circuit fails if it references an undefined parameter.
    #[test]
    fn test_undefined_parameter_error() {
        let workflow = Workflow::new(); // No parameters defined

        let circuit_def = CircuitDef {
            name: "test_circ".to_string(),
            qubits: 1,
            body: vec![
                SymbolicGate { name: "RZ".to_string(), args: vec![Value::Symbol("undefined_angle".to_string()), Value::Num(0.0)] },
            ],
        };

        let result = workflow.build_concrete_circuit(&circuit_def);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Undefined parameter 'undefined_angle'"));
    }

    /// A new test for a simple workflow with one parameter and one RY gate.
    #[test]
    fn test_single_parameter_and_ry_gate() {
        let declarations = vec![
            Declaration::DefParam {
                name: "my_angle".to_string(),
                value: 0.5,
            },
            Declaration::DefCircuit {
                name: "simple_ry".to_string(),
                qubits: 1,
                body: vec![SymbolicGate {
                    name: "RY".to_string(),
                    args: vec![Value::Symbol("my_angle".to_string()), Value::Num(0.0)],
                }],
            },
        ];

        let mut workflow = Workflow::new();
        // Execute the declarations to populate the workflow state.
        workflow.execute(declarations).unwrap();

        // Now, manually build the circuit to check the result.
        let circuit_def = workflow.circuits.get("simple_ry").unwrap();
        let concrete_circuit = workflow.build_concrete_circuit(circuit_def).unwrap();

        assert_eq!(concrete_circuit.len(), 1);
        assert_eq!(concrete_circuit[0], ConcreteGate::RY { qubit: 0, theta: 0.5 });
    }
}
