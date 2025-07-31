use chumsky::Parser;
use chumsky::span::SimpleSpan;

pub mod parser;

// pub use self::parse_qcl_code;
pub use parser::validate_ast;
