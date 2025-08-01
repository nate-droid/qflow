use crate::parser::{qcl_parser, validate_ast};
use crate::workflow::Workflow;
use chumsky::Parser;
use std::fs;
use std::io::{self, Write};

/// Pre-processes the QCL code to remove comments and normalize whitespace.
fn preprocess_qcl(code: &str) -> String {
    code.lines()
        .map(|line| line.split(';').next().unwrap_or("").trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

/// Runs the QCL REPL loop.
pub fn run_repl() {
    println!("==============================================");
    println!(" Welcome to QCL (Quantum Circuit Language) REPL");
    println!(" Version: 0.1.0");
    println!("==============================================");
    println!("Type ':quit' or ':exit' to leave.");
    println!("Type ':load <file>' to load a QCL file.");
    println!("Multi-line input: Enter code, then a single '.' on a line to execute.");
    println!();

    let mut workflow = Workflow::new();
    let mut last_code_block: Option<String> = None;
    let mut history: Vec<String> = Vec::new();

    loop {
        let mut input_lines = Vec::new();
        print!("qcl> ");
        io::stdout().flush().unwrap();

        let mut first_line = String::new();
        if io::stdin().read_line(&mut first_line).is_err() {
            println!("Error reading input.");
            continue;
        }
        let first_line = first_line.trim();

        // Handle REPL commands
        if first_line == ":quit" || first_line == ":exit" {
            println!("Exiting QCL REPL.");
            break;
        } else if first_line.starts_with(":load ") {
            let file_path = first_line[6..].trim();
            match fs::read_to_string(file_path) {
                Ok(content) => {
                    println!("Loaded file '{}'. Executing...", file_path);
                    execute_qcl_block(&content, &mut workflow);
                    last_code_block = Some(content);
                }
                Err(e) => {
                    println!("Failed to read file '{}': {}", file_path, e);
                }
            }
            continue;
        } else if first_line.starts_with(":save ") {
            let file_path = first_line[6..].trim();
            let to_save = match &last_code_block {
                Some(code) => code,
                None => {
                    println!("No code block to save yet.");
                    continue;
                }
            };
            match fs::write(file_path, to_save) {
                Ok(_) => println!("Saved last code block to '{}'.", file_path),
                Err(e) => println!("Failed to save file '{}': {}", file_path, e),
            }
            continue;
        } else if first_line == ":reset" {
            workflow = Workflow::new();
            println!("Workflow state has been reset.");
            continue;
        } else if first_line == ":vars" {
            if workflow.params.is_empty() {
                println!("No parameters defined.");
            } else {
                println!("Current parameters:");
                for (name, value) in &workflow.params {
                    println!("  {} = {}", name, value);
                }
            }
            continue;
        } else if first_line == ":macros" {
            if workflow.macros.is_empty() {
                println!("No macros defined.");
            } else {
                println!("Current macros:");
                for (name, mac) in &workflow.macros {
                    println!("  {}({})", name, mac.params.join(", "));
                }
            }
            continue;
        } else if first_line == ":circuits" {
            if workflow.circuits.is_empty() {
                println!("No circuits defined.");
            } else {
                println!("Current circuits:");
                for (name, circ) in &workflow.circuits {
                    println!(
                        "  {} ({} qubits, {} gates)",
                        name,
                        circ.qubits,
                        circ.body.len()
                    );
                }
            }
            continue;
        } else if first_line == ":obs" {
            if workflow.observables.is_empty() {
                println!("No observables defined.");
            } else {
                println!("Current observables:");
                for (name, obs) in &workflow.observables {
                    println!("  {} = {}", name, obs.operator);
                }
            }
            continue;
        } else if first_line == ":history" {
            if history.is_empty() {
                println!("No history yet.");
            } else {
                println!("Command/code history:");
                for (i, entry) in history.iter().enumerate() {
                    println!("  [{}] {}", i + 1, entry.replace("\n", " "));
                }
            }
            continue;
        } else if first_line == "." {
            // Ignore lone '.' at start
            continue;
        }

        // Multi-line input: keep reading until a single '.' line
        if !first_line.is_empty() {
            input_lines.push(first_line.to_string());
            loop {
                print!("... ");
                io::stdout().flush().unwrap();
                let mut next_line = String::new();
                if io::stdin().read_line(&mut next_line).is_err() {
                    println!("Error reading input.");
                    break;
                }
                let next_line = next_line.trim();
                if next_line == "." {
                    break;
                }
                input_lines.push(next_line.to_string());
            }
        } else {
            continue;
        }

        let block = input_lines.join("\n");
        execute_qcl_block(&block, &mut workflow);
        last_code_block = Some(block.clone());
        history.push(block);
    }
}

/// Executes a block of QCL code in the REPL, printing results/errors.
fn execute_qcl_block(qcl_code: &str, workflow: &mut Workflow) {
    let cleaned_code = preprocess_qcl(qcl_code);

    let result = qcl_parser().parse(&cleaned_code);

    if result.has_errors() {
        println!("--- Parsing Failed ---");
        result.errors().for_each(|e| println!("Error: {}", e));
        return;
    }

    let ast = match result.output() {
        Some(ast) => ast,
        None => {
            println!("--- Parsing produced no AST ---");
            return;
        }
    };

    let declarations = match validate_ast(ast) {
        Ok(decls) => decls,
        Err(e) => {
            println!("--- Validation Failed ---");
            println!("{}", e);
            return;
        }
    };

    // If the block is a single EvalExpr, print only the result (not workflow status)
    if declarations.len() == 1 {
        if let crate::parser::Declaration::EvalExpr(_) = &declarations[0] {
            workflow.run(declarations).ok();
            return;
        }
    }

    if let Err(e) = workflow.run(declarations) {
        println!("--- Workflow Execution Failed ---");
        println!("{}", e);
        return;
    }
    println!("--- Workflow Execution Complete ---");
}
