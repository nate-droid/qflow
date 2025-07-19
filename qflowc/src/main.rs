use chumsky::prelude::*;
use clap::Parser as ClapParser;
use anyhow::{Result, anyhow, Context};
use std::io::Read;
use std::path::PathBuf;

// FIX: Import the renamed types directly without aliasing.
use qflow_types::{QuantumWorkflow, QuantumWorkflowSpec, QFlowTask, QFlowTaskSpec};
use kube::api::ObjectMeta;

// --- 1. Abstract Syntax Tree (AST) ---
#[derive(Debug, Clone)]
pub enum AstTaskSpec {
    Classical { image: String },
    Quantum { image: String, circuit_from: PathBuf, params_from: PathBuf },
}

#[derive(Debug, Clone)]
pub struct AstTask {
    name: String,
    spec: AstTaskSpec,
}

#[derive(Debug, Clone)]
pub struct AstWorkflow {
    name: String,
    tasks: Vec<AstTask>,
}

// --- 2. The Parser (Unchanged logic, but returns new AST types) ---
fn workflow_parser() -> impl Parser<char, AstWorkflow, Error = Simple<char>> {
    let ident = filter(|c: &char| c.is_alphanumeric() || *c == '-')
        .repeated().at_least(1).collect::<String>().padded();

    let string_literal = just('"')
        .ignore_then(filter(|c| *c != '"').repeated())
        .then_ignore(just('"'))
        .collect::<String>().padded();

    // FIX: Define a more robust parser for the task body that handles
    // unordered fields and optional trailing commas.
    #[derive(Clone, Debug)]
    enum Field {
        Image(String),
        Circuit(PathBuf),
        Params(PathBuf),
    }

    let image_field = just("image:").ignore_then(string_literal.clone()).map(Field::Image);
    let circuit_field = just("circuit_from:").ignore_then(string_literal.clone().map(PathBuf::from)).map(Field::Circuit);
    let params_field = just("params_from:").ignore_then(string_literal.clone().map(PathBuf::from)).map(Field::Params);

    let field = choice((image_field, circuit_field, params_field))
        .then_ignore(just(',').or_not().padded());

    let task_body = field.repeated()
        .padded().delimited_by(just('{'), just('}'))
        .try_map(|fields, span| {
            let mut image = None;
            let mut circuit = None;
            let mut params = None;

            for field in fields {
                match field {
                    Field::Image(s) => image = Some(s),
                    Field::Circuit(p) => circuit = Some(p),
                    Field::Params(p) => params = Some(p),
                }
            }

            if let (Some(image), Some(circuit_from), Some(params_from)) = (image.clone(), circuit, params) {
                Ok(AstTaskSpec::Quantum { image, circuit_from, params_from })
            } else if let Some(image) = image {
                Ok(AstTaskSpec::Classical { image })
            } else {
                Err(Simple::custom(span, "A task must have at least an 'image' field."))
            }
        });

    let task = just("task").padded().ignore_then(ident.clone())
        .then(task_body)
        .map(|(name, spec)| AstTask { name, spec })
        .padded();

    let workflow = just("workflow").padded().ignore_then(ident)
        .then(task.repeated().delimited_by(just('{'), just('}')))
        .map(|(name, tasks)| AstWorkflow { name, tasks });

    workflow.padded().then_ignore(end())
}

// --- 3. The Compiler ---
fn compile(ast: AstWorkflow) -> Result<QuantumWorkflow> {
    let tasks = ast.tasks.into_iter()
        .map(|task| -> Result<QFlowTask> {
            // FIX: Match on the AST spec and create the corresponding QFlowTaskSpec.
            let spec = match task.spec {
                AstTaskSpec::Classical { image } => QFlowTaskSpec::Classical { image },
                AstTaskSpec::Quantum { image, circuit_from, params_from } => {
                    let circuit = std::fs::read_to_string(&circuit_from)
                        .with_context(|| format!("Failed to read circuit file: {}", circuit_from.display()))?;
                    let params = std::fs::read_to_string(&params_from)
                        .with_context(|| format!("Failed to read params file: {}", params_from.display()))?;
                    QFlowTaskSpec::Quantum { image, circuit, params }
                }
            };
            // FIX: Construct the renamed QFlowTask struct. This should now compile correctly.
            Ok(QFlowTask { name: task.name, spec })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(QuantumWorkflow {
        metadata: ObjectMeta { name: Some(ast.name), ..Default::default() },
        spec: QuantumWorkflowSpec { tasks },
        status: None,
    })
}

// --- 4. Main Application Logic ---
#[derive(ClapParser, Debug)]
struct Args { #[arg(short, long)] file: Option<String> }

fn main() -> Result<()> {
    let args = Args::parse();
    let mut src = String::new();
    let path = "./qflowc/examples/quantum_test.qflow";
    // if let Some(path) = args.file { src = std::fs::read_to_string(path)?; } else { std::io::stdin().read_to_string(&mut src)?; };
    src = std::fs::read_to_string(path)?;

    let ast = workflow_parser().parse(src).map_err(|e| anyhow!("Parser errors: {:?}", e))?;
    let k8s_resource = compile(ast)?;
    let yaml_output = serde_yaml::to_string(&k8s_resource)?;
    println!("---\n{}", yaml_output);

    Ok(())
}

