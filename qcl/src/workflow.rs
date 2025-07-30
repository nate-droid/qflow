use crate::parser::{Declaration, Gate as SymbolicGate, Value};
use chumsky::span::SimpleSpan;
use qsim::Gate as ConcreteGate; // Your existing, concrete Gate enum from qsim
use std::collections::HashMap;
use std::borrow::Borrow;

// ================================================================================================
// |                                    Workflow State & Definitions                               |
// ================================================================================================

#[derive(Debug, Clone)]
pub struct CircuitDef {
    pub name: String,
    pub qubits: u64,
    pub body: Vec<SymbolicGate>,
}

/// Represents a user-defined macro.
#[derive(Debug, Clone)]
pub struct MacroDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<SymbolicGate>,
}

pub struct Workflow {
    pub params: HashMap<String, f64>,
    pub circuits: HashMap<String, CircuitDef>,
    /// A map to store user-defined macros.
    pub macros: HashMap<String, MacroDef>,
    /// A counter to track the number of simulation runs, useful for testing.
    pub run_counter: u32,
}

// ================================================================================================
// |                                     Execution Logic                                          |
// ================================================================================================

impl Workflow {
    pub fn new() -> Self {
        Workflow {
            params: HashMap::new(),
            circuits: HashMap::new(),
            macros: HashMap::new(),
            run_counter: 0,
        }
    }

    pub fn run(&mut self, declarations: Vec<Declaration>) -> Result<(), String> {
        self.execute(&declarations)
    }

    fn execute(&mut self, declarations: &[Declaration]) -> Result<(), String> {
        for decl in declarations {
            match decl {
                Declaration::DefParam { name, value } => {
                    println!("[Workflow] Defining parameter: '{}' = {}", name, value);
                    self.params.insert(name.clone(), *value);
                }
                Declaration::DefCircuit { name, qubits, body } => {
                    println!("[Workflow] Defining circuit: '{}'", name);
                    let circuit_def = CircuitDef {
                        name: name.clone(),
                        qubits: *qubits,
                        body: body.clone(),
                    };
                    self.circuits.insert(name.clone(), circuit_def);
                }
                Declaration::DefMacro { name, params, body } => {
                    println!("[Workflow] Defining macro: '{}'", name);
                    let macro_def = MacroDef {
                        name: name.clone(),
                        params: params.clone(),
                        body: body.clone(),
                    };
                    self.macros.insert(name.clone(), macro_def);
                }
                Declaration::DefObs { name, operator } => {
                    println!("[Workflow] Defining observable: '{}' = {}", name, operator);
                }
                Declaration::Run(run_args) => {
                    println!("[Workflow] --- Triggering Run ---");
                    self.run_simulation(run_args)?;
                }
                Declaration::Loop { times, body } => {
                    println!("[Workflow] >>> Entering Loop ({} iterations)", times);
                    for i in 0..*times {
                        println!("[Workflow] >> Loop iteration {}", i + 1);
                        self.execute(body)?;
                    }
                    println!("[Workflow] <<< Exiting Loop");
                }
            }
        }
        Ok(())
    }

    fn run_simulation(&mut self, args: &HashMap<String, Value>) -> Result<(), String> {
        let circuit_name = match args.get("circuit") {
            Some(Value::Symbol(s)) => s,
            _ => return Err("Run command must specify a circuit, e.g., (run (circuit: 'my_circ'))".to_string()),
        };

        let circuit_def = self.circuits.get(circuit_name)
            .ok_or_else(|| format!("Circuit '{}' not found for run command", circuit_name))?;

        let run_params = match args.get("with") {
            Some(Value::List(pairs)) => self.parse_run_params(pairs)?,
            Some(_) => return Err("Expected 'with:' argument to be a list of (symbol value) pairs.".to_string()),
            None => HashMap::new(),
        };

        let shots = match args.get("shots") {
            Some(Value::Num(n)) => *n as u64,
            None => 1024,
            _ => return Err("Expected 'shots:' argument to be a number.".to_string()),
        };

        println!("[Workflow] Building concrete circuit for '{}' with {} shots.", circuit_def.name, shots);

        let concrete_circuit = self.build_concrete_circuit(circuit_def, &run_params)?;

        println!("[Workflow] Concrete circuit built with {} gates.", concrete_circuit.len());

        self.run_counter += 1;

        Ok(())
    }

    fn parse_run_params(&self, pairs: &[(Value, SimpleSpan)]) -> Result<HashMap<String, f64>, String> {
        let mut params = HashMap::new();
        for (pair_val, _) in pairs {
            if let Value::List(p) = pair_val {
                if p.len() != 2 { return Err("Parameter override must be a (symbol value) pair".to_string()); }
                let name = match &p[0].0 { Value::Symbol(s) => s.clone(), _ => return Err("Expected symbol for parameter override name".to_string()) };
                let val = match &p[1].0 { Value::Num(n) => *n, _ => return Err("Expected number for parameter override value".to_string()) };
                params.insert(name, val);
            }
        }
        Ok(params)
    }

    fn build_concrete_circuit(&self, circuit_def: &CircuitDef, run_params: &HashMap<String, f64>) -> Result<Vec<ConcreteGate>, String> {
        let mut all_gates = Vec::new();
        for symbolic_gate in &circuit_def.body {
            let concrete_gates = self.expand_and_build_gate(symbolic_gate, run_params)?;
            all_gates.extend(concrete_gates);
        }
        Ok(all_gates)
    }

    fn expand_and_build_gate(&self, symbolic_gate: &SymbolicGate, run_params: &HashMap<String, f64>) -> Result<Vec<ConcreteGate>, String> {
        if let Some(macro_def) = self.macros.get(&symbolic_gate.name) {
            return self.expand_macro(macro_def, &symbolic_gate.args, run_params);
        }

        let concrete_gate = self.build_single_concrete_gate(symbolic_gate, run_params)?;
        Ok(vec![concrete_gate])
    }

    /// Expands a macro into a list of concrete gates.
    fn expand_macro(&self, macro_def: &MacroDef, args: &[Value], run_params: &HashMap<String, f64>) -> Result<Vec<ConcreteGate>, String> {
        if macro_def.params.len() != args.len() {
            return Err(format!("Macro '{}' expects {} arguments, but got {}", macro_def.name, macro_def.params.len(), args.len()));
        }

        // FIX: Create a substitution map from parameter name (&str) to the provided value (&Value).
        let substitutions: HashMap<&str, &Value> = macro_def.params.iter().map(|s| s.as_str()).zip(args.iter()).collect();

        let mut expanded_gates = Vec::new();
        for template_gate in &macro_def.body {
            let substituted_args = template_gate.args.iter().map(|arg| {
                if let Value::Symbol(s) = arg {
                    // FIX: Look up using a &str and dereference the result twice to get the Value.
                    if let Some(subst_val) = substitutions.get(s.as_str()) {
                        return (**subst_val).clone();
                    }
                }
                arg.clone()
            }).collect();

            let new_symbolic_gate = SymbolicGate {
                name: template_gate.name.clone(),
                args: substituted_args,
            };

            expanded_gates.extend(self.expand_and_build_gate(&new_symbolic_gate, run_params)?);
        }

        Ok(expanded_gates)
    }

    fn build_single_concrete_gate(&self, symbolic_gate: &SymbolicGate, run_params: &HashMap<String, f64>) -> Result<ConcreteGate, String> {
        let get_qubit = |arg_idx: usize| -> Result<usize, String> {
            match &symbolic_gate.args.get(arg_idx) {
                Some(Value::Num(n)) => Ok(*n as usize),
                _ => Err(format!("Expected a qubit index (number) for gate '{}'", symbolic_gate.name)),
            }
        };

        let get_angle = |arg_idx: usize| -> Result<f64, String> {
            match &symbolic_gate.args.get(arg_idx) {
                Some(Value::Num(n)) => Ok(*n),
                Some(Value::Symbol(s)) => {
                    run_params.get(s).cloned()
                        .or_else(|| self.params.get(s).cloned())
                        .ok_or_else(|| format!("Undefined parameter '{}' for gate '{}'", s, symbolic_gate.name))
                }
                _ => Err(format!("Invalid argument for angle in gate '{}'", symbolic_gate.name)),
            }
        };

        match symbolic_gate.name.as_str() {
            "H" => Ok(ConcreteGate::H { qubit: get_qubit(0)? }),
            "X" => Ok(ConcreteGate::X { qubit: get_qubit(0)? }),
            "CX" | "CNOT" => Ok(ConcreteGate::CX { control: get_qubit(0)?, target: get_qubit(1)? }),
            "RY" => Ok(ConcreteGate::RY { theta: get_angle(0)?, qubit: get_qubit(1)? }),
            "RZ" => Ok(ConcreteGate::RZ { theta: get_angle(0)?, qubit: get_qubit(1)? }),
            _ => Err(format!("Unknown gate or macro name '{}'", symbolic_gate.name)),
        }
    }
}

// ================================================================================================
// |                                             Tests                                            |
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        let result = workflow.run(declarations);

        assert!(result.is_ok());
        assert_eq!(workflow.params.get("theta"), Some(&1.57));
        assert!(workflow.circuits.contains_key("my_circ"));
        assert_eq!(workflow.circuits.get("my_circ").unwrap().qubits, 1);
    }

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

        let concrete_circuit = workflow.build_concrete_circuit(&circuit_def, &HashMap::new()).unwrap();

        assert_eq!(concrete_circuit.len(), 2);
        assert_eq!(concrete_circuit[0], ConcreteGate::H { qubit: 0 });
        assert_eq!(concrete_circuit[1], ConcreteGate::RY { theta: 3.14, qubit: 1 });
    }

    #[test]
    fn test_undefined_parameter_error() {
        let workflow = Workflow::new();

        let circuit_def = CircuitDef {
            name: "test_circ".to_string(),
            qubits: 1,
            body: vec![
                SymbolicGate { name: "RZ".to_string(), args: vec![Value::Symbol("undefined_angle".to_string()), Value::Num(0.0)] },
            ],
        };

        let result = workflow.build_concrete_circuit(&circuit_def, &HashMap::new());
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Undefined parameter 'undefined_angle'"));
    }

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
        workflow.run(declarations).unwrap();

        let circuit_def = workflow.circuits.get("simple_ry").unwrap();
        let concrete_circuit = workflow.build_concrete_circuit(circuit_def, &HashMap::new()).unwrap();

        assert_eq!(concrete_circuit.len(), 1);
        assert_eq!(concrete_circuit[0], ConcreteGate::RY { theta: 0.5, qubit: 0 });
    }

    #[test]
    fn test_loop_execution() {
        let declarations = vec![
            Declaration::DefCircuit {
                name: "dummy_circ".to_string(),
                qubits: 1,
                body: vec![],
            },
            Declaration::Loop {
                times: 5,
                body: vec![Declaration::Run(
                    [("circuit".to_string(), Value::Symbol("dummy_circ".to_string()))]
                        .iter().cloned().collect()
                )],
            },
        ];

        let mut workflow = Workflow::new();
        workflow.run(declarations).unwrap();

        assert_eq!(workflow.run_counter, 5);
    }

    #[test]
    fn test_nested_loop_execution() {
        let declarations = vec![
            Declaration::DefCircuit {
                name: "dummy_circ".to_string(),
                qubits: 1,
                body: vec![],
            },
            Declaration::Loop {
                times: 3,
                body: vec![
                    Declaration::Loop {
                        times: 4,
                        body: vec![Declaration::Run(
                            [("circuit".to_string(), Value::Symbol("dummy_circ".to_string()))]
                                .iter().cloned().collect()
                        )],
                    }
                ],
            },
        ];

        let mut workflow = Workflow::new();
        workflow.run(declarations).unwrap();

        assert_eq!(workflow.run_counter, 12);
    }

    /// NEW TEST: Verify that a simple macro can be defined and expanded.
    #[test]
    fn test_simple_macro_expansion() {
        let declarations = vec![
            Declaration::DefMacro {
                name: "entangle".to_string(),
                params: vec!["q1".to_string(), "q2".to_string()],
                body: vec![
                    SymbolicGate { name: "H".to_string(), args: vec![Value::Symbol("q1".to_string())] },
                    SymbolicGate { name: "CX".to_string(), args: vec![Value::Symbol("q1".to_string()), Value::Symbol("q2".to_string())] },
                ],
            },
            Declaration::DefCircuit {
                name: "main".to_string(),
                qubits: 2,
                body: vec![
                    SymbolicGate { name: "entangle".to_string(), args: vec![Value::Num(0.0), Value::Num(1.0)] }
                ],
            },
        ];

        let mut workflow = Workflow::new();
        workflow.run(declarations).unwrap();

        let circuit_def = workflow.circuits.get("main").unwrap();
        let concrete_circuit = workflow.build_concrete_circuit(circuit_def, &HashMap::new()).unwrap();

        assert_eq!(concrete_circuit.len(), 2);
        assert_eq!(concrete_circuit[0], ConcreteGate::H { qubit: 0 });
        assert_eq!(concrete_circuit[1], ConcreteGate::CX { control: 0, target: 1 });
    }
}
