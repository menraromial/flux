//! Flux Standard Library
//! 
//! This module provides the core standard library functionality for the Flux programming language,
//! including I/O operations, collections, string manipulation, and other essential utilities.

pub mod io;
pub mod collections;
pub mod string;

// Re-export commonly used items
pub use io::{print, println, read_line, File};
pub use collections::{Array, List, Map, Set};
pub use string::{FluxString, FluxRegex};