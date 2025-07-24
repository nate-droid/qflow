# qflow-operator

The qflow-operator is a Kubernetes operator written in Rust. It is responsible for managing the lifecycle of QFlow 
Custom Resource Definitions (CRDs). The operator will create the necessary resources to run quantum circuits on a 
quantum computer (in this case, the qsim simulator), and then update the CRD with the results. It derives it's types from the 
qflow-crd crate, which defines the QFlow CRDs in a centralized manner.

When a new QFlow CRD is created, the operator will create a Kubernetes Job to run the quantum circuit. The job will have 
a PVC attached to it, which will be used to store the results of the quantum circuit. The operator will then watch for the
completion of the job, and when it is complete, it will update the QFlow CRD with the results.


# Pre-requisites
* Kubernetes cluster (minikube, kind, etc.)
  * Must have a CSI driver installed for PVCs
* Rust toolchain (nightly)