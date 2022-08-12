#![feature(min_specialization)]
#![allow(clippy::option_map_unit_fn)]

mod ast;
pub use ast::*;

mod visitor;
pub use visitor::*;

mod analysis;
pub use analysis::*;

mod lexer;
pub use lexer::*;

mod parser;
pub use parser::*;

#[cfg(test)]
mod test;
