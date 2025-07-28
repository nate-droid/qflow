# QFlow

QFlow is a Kubernetes-native dataflow system for running quantum circuits and integrating their results into classical pipelines. It is designed to be practical, minimal, and easy to explore for developers interested in quantum workflows.

## Project Structure

- **frontend/**: Simple HTML interface for viewing QFlow custom resources and pipeline status.
- **qflow-backend/**: API backend serving pipeline and resource status to the frontend.
- **qflow-operator/**: Kubernetes operator managing QFlow custom resources, running quantum jobs, and updating results.
- **qflowc/**: Compiler for the QFlow DSL. Converts OpenQASM or QFlow DSL files into Kubernetes CRDs for use with the operator.
- **qsim/**: Standalone quantum circuit simulator. Used by the operator, but can be run independently.

## Quickstart

Run a quantum circuit example with the simulator:

```bash
cargo run --bin qsim -- --input-file qsim/examples/bell.qasm --output-file results.json
```

Compile a QFlow DSL file to a CRD and apply it to your cluster:

```bash
cat qflow-operator/tests/dag-test.qflow | cargo run -p qflowc | kubectl apply -f -
```

## Next Steps

- Explore the `examples/` folders in `qsim/` and `qflowc/` for sample circuits and workflows.
- See individual component READMEs for more details.