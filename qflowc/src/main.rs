use chumsky::prelude::*;
use serde::{Serialize};
use clap::Parser as ClapParser;
use anyhow::{Result, anyhow};
use std::io::Read; // Import Read trait for read_to_string
use qflow_types::{Metadata, QuantumWorkflowResource, QuantumWorkflowSpec, Task, TaskSpec};

// --- 1. Abstract Syntax Tree (AST) ---
// These structs represent the logical structure of our `.qflow` language itself.
// The parser's job is to turn source text into this structure.

#[derive(Debug, Clone)]
pub struct Workflow {
    name: String,
    tasks: Vec<Task>,
}

// --- 2. Kubernetes Resource Definitions ---
// These structs are a 1-to-1 mapping of the YAML structure of our CRD.
// The compiler's job is to turn the AST (above) into this structure.
// We use `serde` attributes to control the final YAML output format.



// --- 3. The Parser ---
// This function uses `chumsky` to define the grammar of our v0.1 language.
// Grammar:
//   workflow <name> {
//     task <task_name> {
//       image: "<image_name>"
//     }
//     ...
//   }

fn workflow_parser() -> impl Parser<char, Workflow, Error = Simple<char>> {
    let ident = filter(|c: &char| c.is_alphanumeric() || *c == '-')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .padded();

    let string_literal = just('"')
        .ignore_then(filter(|c| *c != '"').repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .padded();

    let task_body = just("image:")
        .ignore_then(string_literal)
        .padded()
        .delimited_by(just('{'), just('}'));

    // FIX: Add .padded() to the entire task definition. This ensures that after
    // one task is parsed, any trailing whitespace is consumed before trying
    // to parse the next task in the `repeated()` block.
    let task = just("task")
        .padded()
        .ignore_then(ident.clone()) // task name
        .then(task_body)
        .map(|(name, image)| Task { name, image })
        .padded();

    let workflow = just("workflow")
        .padded()
        .ignore_then(ident) // workflow name
        .then(
            // workflow body
            task.repeated().delimited_by(just('{'), just('}'))
        )
        .map(|(name, tasks)| Workflow { name, tasks });

    workflow.padded().then_ignore(end())
}


// --- 4. The Compiler ---
// This function converts our language-specific AST into the Kubernetes resource struct.

fn compile(ast: Workflow) -> QuantumWorkflowResource {
    let tasks: Vec<Task> = ast.tasks.into_iter().map(|task| Task {
        name: task.name,
        image: task.image,
    }).collect();

    QuantumWorkflowResource {
        api_version: "qflow.io/v1alpha1".to_string(),
        kind: "QuantumWorkflow".to_string(),
        metadata: Metadata {
            name: ast.name,
        },
        spec: QuantumWorkflowSpec { tasks },
    }
}

// --- 5. Command-Line Interface ---
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to the .qflow file to compile. Reads from stdin if not provided.
    #[arg(short, long)]
    file: Option<String>,
}


// --- 6. Main Application Logic ---
fn main() -> Result<()> {
    let mut args = Args::parse();
    let path = "qflowc/test.qflow"; // Default path if not provided

    // Read source code from file or stdin
    // let src = if let Some(path) = args.file {
    args.file = Some(path.to_string());

    let src = if let Some(path) = args.file {
        std::fs::read_to_string(path)?
    } else {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    };

    // Parse the source code into an AST
    let ast = workflow_parser().parse(src.clone()).map_err(|e| anyhow!("Parser errors: {:?}", e))?;

    // Compile the AST into a Kubernetes resource
    let k8s_resource = compile(ast);

    // Serialize the resource to YAML and print to stdout
    let yaml_output = serde_yaml::to_string(&k8s_resource)?;
    println!("{}", yaml_output);

    Ok(())
}


// --- 7. Unit Tests ---
// It's crucial to have tests for both the parser and the compiler.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_workflow_parsing() {
        let qflow_code = r#"
            workflow my-first-workflow {
                task hello-world {
                    image: "alpine:latest"
                }
            }
        "#;
        let result = workflow_parser().parse(qflow_code);
        assert!(result.is_ok(), "Parsing failed with error: {:?}", result.err());
        let ast = result.unwrap();
        assert_eq!(ast.name, "my-first-workflow");
        assert_eq!(ast.tasks.len(), 1);
        assert_eq!(ast.tasks[0].name, "hello-world");
        assert_eq!(ast.tasks[0].image, "alpine:latest");
    }

    #[test]
    fn test_multi_task_workflow_parsing() {
        let qflow_code = r#"
            workflow my-multi-task-workflow {
                task task-one {
                    image: "alpine:latest"
                }

                task task-two {
                    image: "ubuntu:latest"
                }
            }
        "#;
        let result = workflow_parser().parse(qflow_code);
        assert!(result.is_ok(), "Parsing multi-task failed with error: {:?}", result.err());
        let ast = result.unwrap();
        assert_eq!(ast.name, "my-multi-task-workflow");
        assert_eq!(ast.tasks.len(), 2);
    }

    #[test]
    fn test_compilation_to_k8s_resource() {
        let ast = Workflow {
            name: "test-wf".to_string(),
            tasks: vec![Task {
                name: "task-1".to_string(),
                image: "ubuntu".to_string(),
            }],
        };
        let resource = compile(ast);
        assert_eq!(resource.api_version, "qflow.io/v1alpha1");
        assert_eq!(resource.kind, "QuantumWorkflow");
        assert_eq!(resource.metadata.name, "test-wf");
        assert_eq!(resource.spec.tasks.len(), 1);
        assert_eq!(resource.spec.tasks[0].name, "task-1");
        assert_eq!(resource.spec.tasks[0].image, "ubuntu");
    }
}
