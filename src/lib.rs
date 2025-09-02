//! Flux Programming Language Compiler
//! 
//! This crate provides the core compiler infrastructure for the Flux programming language,
//! including lexical analysis, parsing, semantic analysis, and code generation.

pub mod error;
pub mod position;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod codegen;
pub mod runtime;
pub mod std;
pub mod package;
pub mod cli;
pub mod tools;
pub mod ffi;

// Re-export core types for convenience
pub use error::*;
pub use position::Position;

// Re-export core types for convenience