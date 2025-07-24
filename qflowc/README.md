# qflowc

qflowc is a compiler for the QFlow DSL. It allows you to take your existing OpenQASM files and compile them into a 
CRD that can be used with the QFlow operator.

One of the main reasons for writing qflowc was to provide users a way to deploy and run their quantum circuits without 
having to learn too much about Kubernetes or CRDs. It hopefully gives a simple way to run openqasm files in a "cloud native" way.

# Compile a QFlow file to a CRD:

In the examples directory, you can find a sample QFlow file named `quantum-test.qflow`. You can compile this file to a CRD.

```bash
cat examples/quantum-test.qflow | cargo run -p qflowc | kubectl apply -f -
```
