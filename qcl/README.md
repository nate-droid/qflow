# QCL: The Quantum Composition Language - A Guide

## Introduction & Philosophy
   
Welcome to QCL, the Quantum Composition Language. QCL is a simple, powerful language designed specifically for defining and executing hybrid quantum-classical workflows. Its primary goal is to make the process of building and testing variational quantum algorithms intuitive and flexible.
The design of QCL is heavily inspired by the philosophies of Lisp and Forth:
Code is Data: The structure of your code is a simple list. This makes the language easy to parse and allows for powerful code-generation and metaprogramming.
Composition over Configuration: Workflows are built by connecting small, single-purpose blocks. You don't configure a single, massive "simulator" object; you compose a series of simple, declarative steps.
Extensibility: The language is designed to be extended by you, the user. You can define new, reusable components that become a part of the language itself, allowing you to build complex abstractions from simple primitives.

## The Grammar: S-expressions
QCL uses a simple syntax called S-expressions (Symbolic Expressions), which is the same syntax used by Lisp. An S-expression is just a list of items enclosed in parentheses.

```
(command argument1 argument2 ...)
```

### Data Types
There are only a few basic data types:
Numbers: Standard floating-point numbers, e.g., 1.57, -0.5, 100.
Strings: Text enclosed in double quotes, used for things like operator definitions, e.g., "Z0 Z1".
Symbols: A name prefixed with a single quote ('). Symbols are used to name and reference other parts of your workflow, like parameters or circuits, e.g., 'theta_A', 'my_circuit'.
Lists: A sequence of other data types, enclosed in parentheses, e.g., (H 0).
### Comments
Comments begin with a semicolon (;) and continue to the end of the line. They are ignored by the parser.
; This is a comment.

(defparam 'my_angle 3.14) ; This part is code, this part is a comment.


## How to Use QCL: Core Commands
You build a workflow by defining a series of components using these core commands.

```
(defparam 'name initial_value)
```

Defines a classical parameter that can be used in your circuits.
'name': A symbol that gives your parameter a unique name.
initial_value: The starting numerical value for this parameter.
Example:
(defparam 'rotation_angle 0.5)


(defcircuit 'name (qubits N) ...gates...)
Defines a quantum circuit.
'name': A symbol that names your circuit.
(qubits N): A list specifying the number of qubits in the circuit.
...gates...: A sequence of gate operations.
Example:
(defcircuit 'my_ansatz (qubits 2)
(H 0)
(CX 0 1)
(RY 'rotation_angle 1) ; Use the parameter we defined!
)


(defobs 'name "operator_string")
Defines a Pauli operator to be measured.
'name': A symbol that names your observable.
"operator_string": A string representation of the Pauli operators (e.g., "Z0", "1.5 * X0 X1").
Example:
(defobs 'z_measurement "Z0")


(run ...)
Triggers the execution of the workflow. The arguments to run specify which components to use.
Example:
(run (circuit: 'my_ansatz'))


4. How to Extend QCL: Metaprogramming
   The most powerful feature of QCL is the ability to define your own reusable components. This is done with the (def ...) command, which is not yet implemented in the parser but is a key part of the language design.
   (def 'new_word' (parameters...) ...body...)
   This command would allow you to create a new "word" or macro that can be used just like a built-in command.
   Example: Creating a Reusable Layer
   Imagine you frequently use a specific two-qubit entanglement layer in your circuits. Instead of writing it out every time, you can define it once:
   ; Define a new command called 'entangle' that takes two qubit indices.
   (def 'entangle (q1 q2)
   (H q1)
   (CX q1 q2)
   )


Now, you can use entangle inside any other circuit definition, making your code cleaner and more modular:
(defcircuit 'deep_circuit (qubits 4)
; Use our new word!
(entangle 0 1)
(entangle 2 3)
)


This is how you build a library of custom components. You start with the simple primitives (H, CX, RY) and compose them into more complex, meaningful operations.
