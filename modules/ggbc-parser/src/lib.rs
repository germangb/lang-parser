//! Parser for the `GGB` (Great Game Boy) programming language.
//!
//! Tools to perform [syntax analysis] on input source code.
//!
//! This is part of the `GGBC` (Great Game Boy Compiler) toolchain.
//!
//! [syntax analysis]: https://en.wikipedia.org/wiki/Syntax_(programming_languages)
pub mod ast;
pub mod error;
pub mod lex;
pub mod span;

pub use crate::ast::{parse, parse_with_context, Ast, ContextBuilder};
