# qsim

qsim is a minimal quantum computer simulator. It has been designed to be unaware of the other components, 
so can be used as a standalone simulator. It is used by the QFlow operator to run quantum circuits.

You can run the provided examples (from the root directory) with:

```bash
cargo run --bin qsim -- --input-file examples/bell.qasm --output-file results.json
```