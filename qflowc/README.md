# qflowc

qflowc is a compiler for the QFlow DSL. It allows you to take your existing OpenQASM files and compile them into a 
CRD that can be used with the QFlow operator.

One of the main reasons for writing qflowc was to provide users a way to deploy and run their quantum circuits without 
having to learn too much about Kubernetes or CRDs. It hopefully gives a simple way to run openqasm files in a "cloud native" way.

# A Simple Example of a QFlow File

```
workflow my-quantum-workflow {
    task run-bell-state {
        image: "qsim",
        circuit_from: "qflowc/examples/bell_circuit.qasm",
        params_from: "qflowc/examples/sim_params.json"
    }
}
```

Running the example through the compliler will produce something like this:

```yaml
apiVersion: qflow.io/v1alpha1
kind: QuantumWorkflow
metadata:
  name: my-quantum-workflow
spec:
  tasks:
    - name: run-bell-state
      quantum:
        image: docker.io/library/qsim:latest
        circuit: |-
          // Bell State Circuit
          OPENQASM 2.0;
          include "qelib1.inc";
          qreg q[2];
          creg c[2];
          h q[0];
          cx q[0],q[1];
          measure q[0] -> c[0];
          measure q[1] -> c[1];
        params: |-
          {
            "shots": 8192,
            "noise_model": "ideal"
          }
```

# Compile a QFlow file to a CRD:

In the examples directory, you can find a sample QFlow file named `quantum-test.qflow`. You can compile this file to a CRD.

```bash
cat examples/quantum-test.qflow | cargo run -p qflowc | kubectl apply -f -
```
