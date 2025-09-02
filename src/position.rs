//! Source code position tracking
//! 
//! Provides utilities for tracking positions in source code for error reporting
//! and debugging information.

use std::fmt;

/// Represents a position in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Byte offset from start of file
    pub offset: usize,
}

impl Position {
    /// Create a new position
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }
    
    /// Create a position at the start of a file
    pub fn start() -> Self {
        Self::new(1, 1, 0)
    }
    
    /// Advance position by one character
    pub fn advance(&mut self, ch: char) {
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.offset += ch.len_utf8();
    }
    
    /// Create a new position advanced by one character
    pub fn advanced(mut self, ch: char) -> Self {
        self.advance(ch);
        self
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Represents a span of source code between two positions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Start position (inclusive)
    pub start: Position,
    /// End position (exclusive)
    pub end: Position,
}

impl Span {
    /// Create a new span
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
    
    /// Create a span covering a single position
    pub fn single(pos: Position) -> Self {
        Self::new(pos, pos)
    }
    
    /// Combine two spans into one that covers both
    pub fn combine(self, other: Span) -> Span {
        let start = if self.start.offset <= other.start.offset {
            self.start
        } else {
            other.start
        };
        
        let end = if self.end.offset >= other.end.offset {
            self.end
        } else {
            other.end
        };
        
        Span::new(start, end)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start.line == self.end.line {
            write!(f, "{}:{}-{}", self.start.line, self.start.column, self.end.column)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}