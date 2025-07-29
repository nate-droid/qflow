use anyhow::{Context, Result, anyhow};
use chumsky::prelude::*;
use clap::Parser as ClapParser;
use std::path::PathBuf;

use kube::api::ObjectMeta;
use qflow_types::{QFlowTask, QFlowTaskSpec, QuantumWorkflow, QuantumWorkflowSpec, VolumeSpec};

#[derive(ClapParser, Debug)]
struct Args {
    #[arg(short, long)]
    file: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let path = args
        .file
        .unwrap_or_else(|| "./qflow-operator/tests/dag-test.qflow".to_string());
    let yaml_output = qflowc::compile_qflow_file(&path)?;
    println!("---\n{}", yaml_output);
    Ok(())
}
