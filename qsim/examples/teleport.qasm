OPENQASM 2.0;
include "qelib1.inc";

// q[0] is the qubit to be teleported (Alice's qubit)
// q[1] is Alice's half of the entangled pair
// q[2] is Bob's half of the entangled pair
qreg q[3];

// c[0] and c[1] are for Alice's measurements
// c[2] is for Bob's final state measurement
creg c[3];

// Create an entangled pair (Bell state) between q[1] and q[2]
h q[1];
cx q[1], q[2];

// --- Alice's operations ---

// Alice entangles her qubit (q[0]) with her half of the pair (q[1])
cx q[0], q[1];
h q[0];

// Alice measures her two qubits
measure q[0] -> c[0];
measure q[1] -> c[1];

// --- Bob's operations (conditional on Alice's measurements) ---

// If Alice's second measurement was 1, Bob applies an X gate
if(c[1]==1) x q[2];
// If Alice's first measurement was 1, Bob applies a Z gate
if(c[0]==1) z q[2];

// To verify, we measure Bob's qubit. It should be in the original
// state of q[0].
measure q[2] -> c[2];
