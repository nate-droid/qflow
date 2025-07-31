mod parser;
mod workflow;
use crate::parser::qcl_parser;
use chumsky::Parser;

fn main() {
    let qcl_code = r#"
        ; QCL Example: A simple VQE workflow

        ; == 1. Define Parameters ==
        (defparam 'theta_A 0.5)
        (defparam 'theta_B -0.5)

        ; == 2. Define Components ==
        (defcircuit 'ansatz (qubits: 2)
            (H 0)
            (CX 0 1)
            (RY 'theta_A 0)
            (RY 'theta_B 1)
        )

        (defobs 'cost_operator "Z0 Z1")

        (run (optimizer: 'my_optimizer' cost: 'total_cost' steps: 100))
    "#;

    println!("Attempting to parse QCL code...\n");

    // In chumsky v0.10+, `parse` returns a `ParseResult` struct.
    let result = qcl_parser().parse(qcl_code);

    // We can check for errors and print them...
    if result.has_errors() {
        println!("--- Parsing Failed ---");
        result
            .errors()
            .for_each(|e| println!("Error: {}", e));
    }

    // ...and we can get the output if parsing was successful.
    if let Some(ast) = result.output() {
        println!("--- Successfully Parsed AST ---");
        println!("{:#?}", ast);
    }
}

#[cfg(test)]
mod tests {
    use super::parser::{qcl_parser, validate_ast, Declaration, Gate, Value};
    use chumsky::Parser;
    use std::collections::HashMap;
    use std::fs;
    use crate::parser;
    use crate::workflow::Workflow;

    /// Pre-processes the QCL code to remove comments and normalize whitespace.
    fn preprocess_qcl(code: &str) -> String {
        code.lines()
            .map(|line| line.split(';').next().unwrap_or("").trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Helper function to run the parser and validator, providing detailed errors on failure.
    fn run_parser_and_validate(qcl_code: &str) -> Result<Vec<Declaration>, String> {
        let cleaned_code = preprocess_qcl(qcl_code);

        let parse_result = qcl_parser().parse(&cleaned_code);
        if parse_result.has_errors() {
            let errors = parse_result
                .errors()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(format!("Parser failed with errors:\n{}", errors));
        }
        validate_ast(parse_result.output().unwrap())
    }

    #[test]
    fn test_example_1_basic_sanity_check() {
        let qcl_code = r#"
            (defparam 'learning_rate 0.1)
            (defcircuit 'bell_state (qubits 2)
                (H 0)
                (CX 0 1)
            )
            (defobs 'simple_obs "Z0")
        "#;

        let ast = run_parser_and_validate(qcl_code)
            .expect("Validation failed when it should have succeeded.");
        assert_eq!(ast.len(), 3, "Expected 3 top-level declarations");

        // Check the first declaration
        match &ast[0] {
            Declaration::DefParam { name, value } => {
                assert_eq!(name, "learning_rate");
                assert_eq!(*value, parser::Value::Num(0.1));
            }
            _ => panic!("Expected a DefParam declaration"),
        }
    }

    #[test]
    fn test_example_2_vqe_style_circuit() {
        let qcl_code = r#"
            (defparam 'theta_1 0.785)
            (defcircuit 'vqe_ansatz (qubits 2)
                (RY 'theta_1 0)
                (RZ 'theta_2 1)
            )
        "#;

        let ast = run_parser_and_validate(qcl_code)
            .expect("Validation failed when it should have succeeded.");
        assert_eq!(ast.len(), 2);

        // Check the circuit declaration
        match &ast[1] {
            Declaration::DefCircuit { name, qubits, body } => {
                assert_eq!(name, "vqe_ansatz");
                assert_eq!(*qubits, 2);
                assert_eq!(body.len(), 2);
                // Check that the gate arguments were parsed correctly
                assert_eq!(body[0].args[0], Value::Symbol("theta_1".to_string()));
            }
            _ => panic!("Expected a DefCircuit declaration"),
        }
    }

    #[test]
    fn test_example_3_invalid_semantic_error() {
        let qcl_code = r#"
            (defparam 'alpha) ; ERROR: Missing initial value!
        "#;

        // The pre-processor will clean the code, so the parser will receive `(defparam 'alpha)`.
        // This is syntactically valid, but semantically invalid.
        let validation_result = run_parser_and_validate(qcl_code);
        assert!(validation_result.is_err(), "Validator should have failed but didn't");

        // Check for the expected error message
        let error_message = validation_result.err().unwrap();
        assert!(error_message.contains("'defparam' expects 2 arguments"));
    }

    #[test]
    fn test_example_5_invalid_syntax_error() {
        let qcl_code = r#"
            ; Mismatched parenthesis
            (defparam 'mismatch 0.5
        "#;

        // The pre-processor will clean the code, so the parser will receive `(defparam 'mismatch 0.5`.
        // This is syntactically invalid.
        let validation_result = run_parser_and_validate(qcl_code);
        assert!(validation_result.is_err());
        assert!(validation_result.err().unwrap().contains("Parser failed"));
    }

    #[test]
    fn test_example_5_unknown_command() {
        let qcl_code = r#"
            (deffoo 'my_param 1.0)
        "#;

        let validation_result = run_parser_and_validate(qcl_code);
        assert!(validation_result.is_err());

        let error_message = validation_result.err().unwrap();
        assert!(error_message.contains("Unknown command 'deffoo'"));
    }

    #[test]
    fn test_e2e() {
        let angle_file = "angle.txt";
        let energy_file = "last_energy.txt";
        fs::write(angle_file, "0.5").unwrap();

        let content = fs::read_to_string("examples/vqe_step.qcl").unwrap();
        let ast = run_parser_and_validate(&content)
            .expect("Validation failed when it should have succeeded.");

        // 3. EXECUTE: Create a workflow and run the AST.
        let mut workflow = Workflow::new();
        workflow.run(ast).expect("Workflow execution failed");

        // 4. ASSERT: Check that the files were updated correctly.
        let updated_angle_content = fs::read_to_string(angle_file).unwrap();
        // 0.5 - (0.1 * 1.0) = 0.5 - 0.05 = 0.4
        assert_eq!(updated_angle_content, "0.4");

        let energy_content = fs::read_to_string(energy_file).unwrap();
        assert_eq!(energy_content, "1");

        // 5. CLEANUP
        fs::remove_file(angle_file).unwrap();
        fs::remove_file(energy_file).unwrap();
    }
}