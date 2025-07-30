use chumsky::extra;
use chumsky::prelude::*;
use chumsky::span::SimpleSpan;
use std::collections::HashMap;
// The direct import of qsim::Gate is removed from the parser.
// The interpreter will handle the conversion to qsim types later.

// ================================================================================================
// |                                  Abstract Syntax Tree (AST)                                  |
// ================================================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Str(String),
    Symbol(String),
    List(Vec<(Value, SimpleSpan)>),
}

/// A simple, symbolic representation of a gate, perfect for the AST.
/// It stores the gate's name and its arguments exactly as they appear in the code.
#[derive(Debug, Clone, PartialEq)]
pub struct Gate {
    pub name: String,
    pub args: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    DefParam { name: String, value: f64 },
    DefCircuit {
        name: String,
        qubits: u64,
        // The body now uses our local, symbolic Gate struct.
        body: Vec<Gate>,
    },
    DefObs { name: String, operator: String },
    Run(HashMap<String, Value>),
    // Other Declaration types from the spec would go here.
}

// ================================================================================================
// |                                       Chumsky Parser                                         |
// ================================================================================================

/// This parser is now only responsible for parsing the S-expression syntax.
/// It produces a raw, untyped Abstract Syntax Tree made of `Value` enums.
///
/// NOTE: This parser now assumes the input string has been pre-processed.
/// Comments (prefixed with ';') should be removed, and all whitespace
/// should be normalized to single spaces before calling this parser.
pub fn qcl_parser<'a>() -> impl Parser<'a, &'a str, Vec<(Value, SimpleSpan)>, extra::Err<Simple<'a, char>>> {
    let sexpr_with_span = recursive(|sexpr_with_span| {
        let num = text::int(10)
            .then(just('.').then(text::digits(10)).or_not())
            .to_slice()
            .from_str()
            .unwrapped()
            .map(Value::Num);

        let symbol = just('\'').ignore_then(text::ident().map(String::from)).map(Value::Symbol);

        let str_lit = just('"')
            .ignore_then(none_of('"').repeated().to_slice())
            .then_ignore(just('"'))
            .map(|s: &str| Value::Str(s.to_string()));

        let ident = text::ident().map(|s: &str| Value::Str(s.to_string()));

        // An atom is one of the basic, non-list types.
        let atom = num.or(str_lit).or(symbol).or(ident);

        // A list is a recursive collection of S-expressions, delimited by parentheses.
        // We use the built-in `padded()` to handle whitespace between items.
        let list = sexpr_with_span
            .padded()
            .repeated()
            .collect::<Vec<_>>()
            .delimited_by(just('('), just(')'))
            .map(Value::List);

        // An S-expression is either an atom or a list.
        // We map with span to track the location of each element for better errors.
        atom.or(list)
            .map_with(|v, e| (v, e.span()))
    });

    // The top-level parser is just a sequence of S-expressions, with optional padding.
    sexpr_with_span
        .padded()
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(end())
}

// ================================================================================================
// |                                      Semantic Validation                                     |
// ================================================================================================

/// This function takes the raw output from the parser and validates its meaning,
/// converting it into the final, strongly-typed `Declaration` AST.
pub fn validate_ast(raw_s_exprs: &[(Value, SimpleSpan)]) -> Result<Vec<Declaration>, String> {
    raw_s_exprs
        .iter()
        .map(|(val, span)| try_decl_from_value(val.clone(), *span))
        .collect()
}

/// A new helper function dedicated to parsing a single gate.
fn try_gate_from_value(gate_val: &(Value, SimpleSpan)) -> Result<Gate, String> {
    if let Value::List(gate_items) = &gate_val.0 {
        if gate_items.is_empty() {
            return Err("Gate definition cannot be an empty list".to_string());
        }
        let gate_name = match &gate_items[0].0 {
            Value::Str(s) => s.clone(),
            _ => return Err("Expected gate name as a string".to_string()),
        };
        let args = gate_items[1..].iter().map(|(arg, _)| arg.clone()).collect();
        Ok(Gate { name: gate_name, args })
    } else {
        Err("Expected a list for a gate definition".to_string())
    }
}

// This function produces a simple String error on failure.
fn try_decl_from_value(val: Value, _span: SimpleSpan) -> Result<Declaration, String> {
    let list = match val {
        Value::List(list) => list,
        _ => return Err("Expected a list for a top-level declaration".to_string()),
    };

    if list.is_empty() {
        return Err("Expected a non-empty list for a declaration".to_string());
    }

    let (command_val, command_span) = &list[0];
    let command = match command_val {
        Value::Str(s) => s.as_str(),
        _ => return Err(format!("Expected a command name as the first element at span {:?}", command_span)),
    };

    match command {
        "defparam" => {
            if list.len() != 3 { return Err("'defparam' expects 2 arguments".to_string()); }
            let name = match &list[1].0 { Value::Symbol(s) => s.clone(), _ => return Err("Expected a symbol for parameter name".to_string()) };
            let value = match &list[2].0 { Value::Num(n) => *n, _ => return Err("Expected a number for parameter value".to_string()) };
            Ok(Declaration::DefParam { name, value })
        }
        "defobs" => {
            if list.len() != 3 { return Err("'defobs' expects 2 arguments".to_string()); }
            let name = match &list[1].0 { Value::Symbol(s) => s.clone(), _ => return Err("Expected a symbol for observable name".to_string()) };
            let operator = match &list[2].0 { Value::Str(s) => s.clone(), _ => return Err("Expected a string for the operator".to_string()) };
            Ok(Declaration::DefObs { name, operator })
        }
        "defcircuit" => {
            if list.len() < 3 { return Err("'defcircuit' requires a name, args, and body".to_string()); }
            let name = match &list[1].0 { Value::Symbol(s) => s.clone(), _ => return Err("Expected a symbol for circuit name".to_string()) };

            let (qubits_list, qubits_span) = match &list[2] {
                (Value::List(l), span) => (l, span),
                (_, span) => return Err(format!("Expected a list for qubits declaration at span {:?}", span)),
            };
            if qubits_list.len() != 2 { return Err(format!("Expected (qubits <number>) at span {:?}", qubits_span)); }
            match &qubits_list[0].0 {
                Value::Str(s) if s == "qubits" => (),
                _ => return Err(format!("Expected 'qubits' keyword at span {:?}", qubits_list[0].1)),
            };
            let qubits = match &qubits_list[1].0 {
                Value::Num(n) => *n as u64,
                _ => return Err(format!("Expected a number for qubit count at span {:?}", qubits_list[1].1)),
            };

            // The circuit body parsing is now much cleaner, calling the new helper function.
            let body = list[3..]
                .iter()
                .map(try_gate_from_value)
                .collect::<Result<_, _>>()?;

            Ok(Declaration::DefCircuit { name, qubits, body })
        }
        "run" => {
            Ok(Declaration::Run(HashMap::new()))
        }
        _ => Err(format!("Unknown command '{}'", command)),
    }
}
