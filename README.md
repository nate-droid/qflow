# QFlow

Is a sort of "Quantum Aware" Data Flow operator for Kubernetes. It allows you to run quantum circuits on a quantum 
computer, and then use the results in your data flow pipelines.

It is composed of several components that are detailed below.

# frontend

Simple HTML page that allows you to view your QFlow CRDs and see the status of their pipelines.

# QFlow Backend

A backend API that serves requests to the frontend. This will expose the status of the QFlow CRDs.

# QFlow Operator

A Kubernetes operator that manages the lifecycle of QFlow CRDs. It will create the necessary resources to run your quantum circuits on a quantum computer (in this case just the qflow simulator), and then update the CRD with the results.

# QFlowc

A compiler for the QFlow DSL. This allows you to take your existing openqasm files and compile them into a CRD that can be used with the QFlow operator.

Compile a qflow file to a CRD:

```bash
cat dag-test.qflow | cargo run -p qflowc | kubectl apply -f -
```

# qsim

A minimal quantum computer simulator. It has been designed to be unaware of any of the other components, so can be used as a standalone simulator. It is used by the QFlow operator to run quantum circuits.

You can run the provided examples (from the root directory) with:

```bash
cargo run --bin qsim -- --input-file qsim/examples/bell.qasm --output-file results.json
```

# vqa-runner

(Work in progress) A runner for Variational Quantum Algorithms (VQAs). It will take a QFlow CRD and run the VQA on it, updating the CRD with the results.