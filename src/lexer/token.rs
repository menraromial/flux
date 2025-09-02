//! Token definitions for the Flux language
//! 
//! Defines all token types that can be produced by the lexer.

use std::fmt;

/// Represents a token in the Flux language
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Character(char),
    
    // Keywords
    Let,
    Const,
    Func,
    Struct,
    Class,
    If,
    Else,
    Match,
    Case,
    For,
    While,
    Loop,
    Break,
    Continue,
    Return,
    Go,
    Async,
    Await,
    Import,
    Package,
    Pub,
    Mut,
    Extern,
    
    // Operators
    Plus,           // +
    Minus,          // -
    Multiply,       // *
    Divide,         // /
    Modulo,         // %
    Equal,          // ==
    NotEqual,       // !=
    Less,           // <
    Greater,        // >
    LessEqual,      // <=
    GreaterEqual,   // >=
    And,            // &&
    Or,             // ||
    Not,            // !
    BitwiseAnd,     // &
    BitwiseOr,      // |
    BitwiseXor,     // ^
    BitwiseNot,     // ~
    LeftShift,      // <<
    RightShift,     // >>
    
    // Assignment operators
    Assign,         // =
    PlusAssign,     // +=
    MinusAssign,    // -=
    MultiplyAssign, // *=
    DivideAssign,   // /=
    ModuloAssign,   // %=
    
    // Delimiters
    LeftParen,      // (
    RightParen,     // )
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    
    // Punctuation
    Comma,          // ,
    Semicolon,      // ;
    Colon,          // :
    DoubleColon,    // ::
    Dot,            // .
    Arrow,          // ->
    FatArrow,       // =>
    Question,       // ?
    
    // Special
    Identifier(String),
    Newline,
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Integer(n) => write!(f, "{}", n),
            Token::Float(n) => write!(f, "{}", n),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Boolean(b) => write!(f, "{}", b),
            Token::Character(c) => write!(f, "'{}'", c),
            
            Token::Let => write!(f, "let"),
            Token::Const => write!(f, "const"),
            Token::Func => write!(f, "func"),
            Token::Struct => write!(f, "struct"),
            Token::Class => write!(f, "class"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Match => write!(f, "match"),
            Token::Case => write!(f, "case"),
            Token::For => write!(f, "for"),
            Token::While => write!(f, "while"),
            Token::Loop => write!(f, "loop"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Return => write!(f, "return"),
            Token::Go => write!(f, "go"),
            Token::Async => write!(f, "async"),
            Token::Await => write!(f, "await"),
            Token::Import => write!(f, "import"),
            Token::Package => write!(f, "package"),
            Token::Pub => write!(f, "pub"),
            Token::Mut => write!(f, "mut"),
            Token::Extern => write!(f, "extern"),
            
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Multiply => write!(f, "*"),
            Token::Divide => write!(f, "/"),
            Token::Modulo => write!(f, "%"),
            Token::Equal => write!(f, "=="),
            Token::NotEqual => write!(f, "!="),
            Token::Less => write!(f, "<"),
            Token::Greater => write!(f, ">"),
            Token::LessEqual => write!(f, "<="),
            Token::GreaterEqual => write!(f, ">="),
            Token::And => write!(f, "&&"),
            Token::Or => write!(f, "||"),
            Token::Not => write!(f, "!"),
            Token::BitwiseAnd => write!(f, "&"),
            Token::BitwiseOr => write!(f, "|"),
            Token::BitwiseXor => write!(f, "^"),
            Token::BitwiseNot => write!(f, "~"),
            Token::LeftShift => write!(f, "<<"),
            Token::RightShift => write!(f, ">>"),
            
            Token::Assign => write!(f, "="),
            Token::PlusAssign => write!(f, "+="),
            Token::MinusAssign => write!(f, "-="),
            Token::MultiplyAssign => write!(f, "*="),
            Token::DivideAssign => write!(f, "/="),
            Token::ModuloAssign => write!(f, "%="),
            
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftBracket => write!(f, "["),
            Token::RightBracket => write!(f, "]"),
            
            Token::Comma => write!(f, ","),
            Token::Semicolon => write!(f, ";"),
            Token::Colon => write!(f, ":"),
            Token::DoubleColon => write!(f, "::"),
            Token::Dot => write!(f, "."),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::Question => write!(f, "?"),
            
            Token::Identifier(name) => write!(f, "{}", name),
            Token::Newline => write!(f, "\\n"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}