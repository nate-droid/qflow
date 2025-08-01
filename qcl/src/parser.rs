use chumsky::extra;
use chumsky::prelude::*;
use chumsky::span::SimpleSpan;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Str(String),
    Symbol(String),
    List(Vec<(Value, SimpleSpan)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gate {
    pub name: String,
    pub args: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    DefParam {
        name: String,
        value: Value,
    },
    Let {
        name: String,
        value: Value,
    },
    WriteFile {
        path: String,
        value: Value,
    },
    DefCircuit {
        name: String,
        qubits: u64,
        body: Vec<Gate>,
    },
    DefObs {
        name: String,
        operator: String,
    },
    DefMacro {
        name: String,
        params: Vec<String>,
        body: Vec<Gate>,
    },
    Run(HashMap<String, Value>),
    Loop {
        times: u64,
        body: Vec<Declaration>,
    },
    EvalExpr(Value),
}

pub fn qcl_parser<'a>()
-> impl Parser<'a, &'a str, Vec<(Value, SimpleSpan)>, extra::Err<Simple<'a, char>>> {
    let sexpr_with_span = recursive(|sexpr_with_span| {
        let num = text::int(10)
            .then(just('.').then(text::digits(10)).or_not())
            .to_slice()
            .from_str()
            .unwrapped()
            .map(Value::Num);

        let symbol = just('\'')
            .ignore_then(text::ident().map(String::from))
            .map(Value::Symbol);

        let str_lit = just('"')
            .ignore_then(none_of('"').repeated().to_slice())
            .then_ignore(just('"'))
            .map(|s: &str| Value::Str(s.to_string()));

        let word_parser = text::ident()
            .then(just('-').then(text::ident()).repeated())
            .to_slice();

        // An identifier can be a normal word, a keyword ending in a colon, or an operator.
        let keyword = word_parser.clone().then(just(':')).to_slice();
        let operator = one_of("+-*/").repeated().at_least(1).to_slice();

        // The order is important: try the more specific `keyword` before the general `word_parser`.
        let ident = keyword
            .or(word_parser)
            .or(operator)
            .map(|s: &str| Value::Str(s.to_string()));

        let atom = num.or(str_lit).or(symbol).or(ident);

        let list = sexpr_with_span
            .padded()
            .repeated()
            .collect::<Vec<_>>()
            .delimited_by(just('('), just(')'))
            .map(Value::List);

        atom.or(list).map_with(|v, e| (v, e.span()))
    });

    sexpr_with_span
        .padded()
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(end())
}

pub fn validate_ast(raw_s_exprs: &[(Value, SimpleSpan)]) -> Result<Vec<Declaration>, String> {
    raw_s_exprs
        .iter()
        .map(|(val, span)| try_decl_from_value(val.clone(), *span))
        .collect()
}

fn try_gate_from_value(gate_val: &(Value, SimpleSpan)) -> Result<Gate, String> {
    if let Value::List(gate_items) = &gate_val.0 {
        if gate_items.is_empty() {
            return Err("Gate definition cannot be an empty list".to_string());
        }
        let gate_name = match &gate_items[0].0 {
            Value::Str(s) => s.clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Err("Expected gate name as a string or symbol".to_string()),
        };
        let args = gate_items[1..].iter().map(|(arg, _)| arg.clone()).collect();
        Ok(Gate {
            name: gate_name,
            args,
        })
    } else {
        Err("Expected a list for a gate definition".to_string())
    }
}

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
        _ => {
            return Err(format!(
                "Expected a command name as the first element at span {:?}",
                command_span
            ));
        }
    };

    match command {
        "defparam" => {
            if list.len() != 3 {
                return Err("'defparam' expects 2 arguments".to_string());
            }
            let name = match &list[1].0 {
                Value::Symbol(s) => s.clone(),
                _ => return Err("Expected a symbol for parameter name".to_string()),
            };
            let value = list[2].0.clone();
            Ok(Declaration::DefParam { name, value })
        }
        "let" => {
            if list.len() != 3 {
                return Err("'let' expects 2 arguments: a name and a value expression".to_string());
            }
            let name = match &list[1].0 {
                Value::Symbol(s) => s.clone(),
                _ => return Err("Expected a symbol for let binding name".to_string()),
            };
            let value = list[2].0.clone();
            Ok(Declaration::Let { name, value })
        }
        "write-file" => {
            if list.len() != 3 {
                return Err("'write-file' expects 2 arguments: a path and a value".to_string());
            }
            let path = match &list[1].0 {
                Value::Str(s) => s.clone(),
                _ => return Err("Expected a string for the file path in 'write-file'".to_string()),
            };
            let value = list[2].0.clone();
            Ok(Declaration::WriteFile { path, value })
        }
        "defobs" => {
            if list.len() != 3 {
                return Err("'defobs' expects 2 arguments".to_string());
            }
            let name = match &list[1].0 {
                Value::Symbol(s) => s.clone(),
                _ => return Err("Expected a symbol for observable name".to_string()),
            };
            let operator = match &list[2].0 {
                Value::Str(s) => s.clone(),
                _ => return Err("Expected a string for the operator".to_string()),
            };
            Ok(Declaration::DefObs { name, operator })
        }
        "defcircuit" => {
            if list.len() < 3 {
                return Err("'defcircuit' requires a name, args, and body".to_string());
            }
            let name = match &list[1].0 {
                Value::Symbol(s) => s.clone(),
                _ => return Err("Expected a symbol for circuit name".to_string()),
            };

            let (qubits_list, qubits_span) = match &list[2] {
                (Value::List(l), span) => (l, span),
                (_, span) => {
                    return Err(format!(
                        "Expected a list for qubits declaration at span {:?}",
                        span
                    ));
                }
            };
            if qubits_list.len() != 2 {
                return Err(format!(
                    "Expected (qubits <number>) at span {:?}",
                    qubits_span
                ));
            }
            match &qubits_list[0].0 {
                Value::Str(s) if s == "qubits" => (),
                _ => {
                    return Err(format!(
                        "Expected 'qubits' keyword at span {:?}",
                        qubits_list[0].1
                    ));
                }
            };
            let qubits = match &qubits_list[1].0 {
                Value::Num(n) => *n as u64,
                _ => {
                    return Err(format!(
                        "Expected a number for qubit count at span {:?}",
                        qubits_list[1].1
                    ));
                }
            };

            let body = list[3..]
                .iter()
                .map(try_gate_from_value)
                .collect::<Result<_, _>>()?;

            Ok(Declaration::DefCircuit { name, qubits, body })
        }
        "def" => {
            if list.len() < 3 {
                return Err("'def' requires a name, parameter list, and body".to_string());
            }
            let name = match &list[1].0 {
                Value::Symbol(s) => s.clone(),
                _ => return Err("Expected a symbol for macro name".to_string()),
            };

            let params_list = match &list[2].0 {
                Value::List(l) => l,
                _ => return Err("Expected a list of symbols for macro parameters".to_string()),
            };
            let params = params_list
                .iter()
                .map(|(p, _)| match p {
                    Value::Symbol(s) => Ok(s.clone()),
                    _ => Err("Macro parameters must be symbols".to_string()),
                })
                .collect::<Result<Vec<_>, _>>()?;

            let body = list[3..]
                .iter()
                .map(try_gate_from_value)
                .collect::<Result<_, _>>()?;

            Ok(Declaration::DefMacro { name, params, body })
        }
        "run" => {
            let mut run_args = HashMap::new();
            for arg_pair in &list[1..] {
                if let (Value::List(pair), _) = arg_pair {
                    if pair.len() != 2 {
                        return Err("Run argument should be a (key: value) pair".to_string());
                    }

                    let key = match &pair[0].0 {
                        Value::Str(s) => s.trim_end_matches(':').to_string(),
                        _ => {
                            return Err(
                                "Expected a keyword key (e.g., 'circuit:') for run argument"
                                    .to_string(),
                            );
                        }
                    };

                    let value = pair[1].0.clone();
                    run_args.insert(key, value);
                } else {
                    return Err("Expected a list for a run command argument".to_string());
                }
            }
            Ok(Declaration::Run(run_args))
        }
        "loop" => {
            if list.len() < 2 {
                return Err("'loop' requires arguments and a body".to_string());
            }

            let (times_list, _) = match &list[1] {
                (Value::List(l), span) => (l, span),
                _ => return Err("Expected a list for loop arguments, e.g., (times 10)".to_string()),
            };
            if times_list.len() != 2 {
                if let Value::Str(s) = &times_list[0].0 {
                    if s != "times" {
                        return Err("Expected loop argument to be (times <number>)".to_string());
                    }
                } else {
                    return Err("Expected loop argument to be (times <number>)".to_string());
                }
            }
            let times = match &times_list[1].0 {
                Value::Num(n) => *n as u64,
                _ => return Err("Expected a number for loop times".to_string()),
            };

            let body_s_exprs: Vec<(Value, SimpleSpan)> = list[2..].to_vec();
            let body_decls = validate_ast(&body_s_exprs)?;

            Ok(Declaration::Loop {
                times,
                body: body_decls,
            })
        }
        // If not a known command, treat as EvalExpr for direct evaluation
        _ => Ok(Declaration::EvalExpr(Value::List(list))),
    }
}
