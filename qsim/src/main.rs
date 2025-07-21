use clap::Parser;
use qsim::run_simulation;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::PathBuf;

/// A minimalistic quantum computer simulator in Rust
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The input OpenQASM file to simulate. If not provided, reads from stdin.
    #[arg(short, long)]
    input_file: Option<PathBuf>,

    /// The output file to write JSON results to. If not provided, writes to stdout.
    #[arg(short, long)]
    output_file: Option<PathBuf>,
}

pub fn run_cli() -> io::Result<Option<String>> {
    let cli = Cli::parse();

    let mut qasm_input = String::new();
    if let Some(input_path) = cli.input_file {
        qasm_input = fs::read_to_string(input_path)?;
    } else {
        io::stdin().read_to_string(&mut qasm_input)?;
    }

    if let Some(events) = run_simulation(&qasm_input) {
        let json_output = serde_json::to_string_pretty(&events)
            .expect("Failed to serialize simulation result to JSON.");

        if let Some(output_path) = cli.output_file {
            let file = File::create(output_path)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(json_output.as_bytes())?;
            Ok(None)
        } else {
            Ok(Some(json_output))
        }
    } else {
        Ok(None)
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    println!("starting a QFlow job");

    // Read the QASM input from a file or stdin
    let mut qasm_input = String::new();
    if let Some(input_path) = cli.input_file {
        qasm_input = fs::read_to_string(input_path)?;
    } else {
        println!("Reading QASM from stdin. Press Ctrl+D (or Ctrl+Z on Windows) to end.");
        io::stdin().read_to_string(&mut qasm_input)?;
    }
    println!("attempting to run: \n {:?}", qasm_input);

    // Determine the output writer (file or stdout)
    if let Some(events) = run_simulation(&qasm_input) {
        // Serialize the entire event vector into a single JSON string
        let json_output = serde_json::to_string_pretty(&events)
            .expect("Failed to serialize simulation result to JSON.");

        // Determine the output writer (file or stdout) and write the result
        if let Some(output_path) = cli.output_file {
            let file = File::create(output_path)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(json_output.as_bytes())?;
        } else {
            println!("{}", json_output);
        }
    }

    Ok(())
}
