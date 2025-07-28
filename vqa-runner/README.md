# VQA-runner

This directory is used to run various VQA/VQE experiments.

Right now, this directory hasn't been optimized to run in containers, so is only really testable via the `cargo test` command.
Once the operator structure has settled down a bit, I will spend some time making the runners a bit smoother and kube friendly.

NB: I'll need to rename this, as I've added initial support for Quantum Circuit Born Machines (QCBM) experiments.