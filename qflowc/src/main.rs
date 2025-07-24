use chumsky::prelude::*;
use clap::Parser as ClapParser;
use anyhow::{Result, anyhow, Context};
use std::path::PathBuf;


use qflow_types::{QuantumWorkflow, QuantumWorkflowSpec, QFlowTask, QFlowTaskSpec, VolumeSpec};
use kube::api::ObjectMeta;

#[derive(ClapParser, Debug)]
struct Args { #[arg(short, long)] file: Option<String> }

fn main() -> Result<()> {
    let args = Args::parse();
    let path = args.file.unwrap_or_else(|| "./qflow-operator/tests/dag-test.qflow".to_string());
    let yaml_output = qflowc::compile_qflow_file(&path)?;
    println!("---\n{}", yaml_output);
    Ok(())
}

