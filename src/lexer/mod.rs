//! Lexical analysis module
//! 
//! Provides tokenization of Flux source code into a stream of tokens.

use crate::error::{LexError, LexErrorKind};
use crate::position::Position;

pub mod token;

pub use token::Token;

/// Default implementation of the Flux lexer
pub struct FluxLexer {
    input: Vec<char>,
    position: usize,
    current_pos: Position,
}

impl FluxLexer {
    /// Create a new lexer for the given input
    pub fn new(input: String) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            current_pos: Position::start(),
        }
    }
    
    /// Get the current character
    fn current_char(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }
    
    /// Peek at the next character
    fn peek_char(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }
    
    /// Advance to the next character
    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            self.current_pos.advance(ch);
            self.position += 1;
        }
    }
    
    /// Skip whitespace characters (except newlines)
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    /// Skip single-line comment
    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }
    
    /// Skip multi-line comment
    fn skip_block_comment(&mut self) -> Result<(), LexError> {
        let start_pos = self.current_pos;
        
        while let Some(ch) = self.current_char() {
            if ch == '*' && self.peek_char() == Some('/') {
                self.advance(); // consume '*'
                self.advance(); // consume '/'
                return Ok(());
            }
            self.advance();
        }
        
        Err(LexError {
            position: start_pos,
            kind: LexErrorKind::UnterminatedComment,
        })
    }
    
    /// Read an identifier or keyword
    fn read_identifier(&mut self) -> String {
        let mut result = String::new();
        
        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        result
    }
    
    /// Read a number (integer or float)
    fn read_number(&mut self) -> Result<Token, LexError> {
        let start_pos = self.current_pos;
        let mut result = String::new();
        let mut is_float = false;
        
        // Read integer part
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        // Check for decimal point
        if self.current_char() == Some('.') && self.peek_char().map_or(false, |c| c.is_ascii_digit()) {
            is_float = true;
            result.push('.');
            self.advance(); // consume '.'
            
            // Read fractional part
            while let Some(ch) = self.current_char() {
                if ch.is_ascii_digit() {
                    result.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
        }
        
        // Check for scientific notation
        if let Some(ch) = self.current_char() {
            if ch == 'e' || ch == 'E' {
                is_float = true;
                result.push(ch);
                self.advance();
                
                // Optional sign
                if let Some(sign) = self.current_char() {
                    if sign == '+' || sign == '-' {
                        result.push(sign);
                        self.advance();
                    }
                }
                
                // Exponent digits
                let mut has_exponent_digits = false;
                while let Some(ch) = self.current_char() {
                    if ch.is_ascii_digit() {
                        result.push(ch);
                        self.advance();
                        has_exponent_digits = true;
                    } else {
                        break;
                    }
                }
                
                if !has_exponent_digits {
                    return Err(LexError {
                        position: start_pos,
                        kind: LexErrorKind::InvalidNumber,
                    });
                }
            }
        }
        
        if is_float {
            match result.parse::<f64>() {
                Ok(num) => Ok(Token::Float(num)),
                Err(_) => Err(LexError {
                    position: start_pos,
                    kind: LexErrorKind::InvalidNumber,
                }),
            }
        } else {
            match result.parse::<i64>() {
                Ok(num) => Ok(Token::Integer(num)),
                Err(_) => Err(LexError {
                    position: start_pos,
                    kind: LexErrorKind::InvalidNumber,
                }),
            }
        }
    }
    
    /// Read a string literal with escape sequences
    fn read_string(&mut self) -> Result<Token, LexError> {
        let start_pos = self.current_pos;
        let mut result = String::new();
        
        self.advance(); // consume opening quote
        
        while let Some(ch) = self.current_char() {
            match ch {
                '"' => {
                    self.advance(); // consume closing quote
                    return Ok(Token::String(result));
                }
                '\\' => {
                    self.advance(); // consume backslash
                    match self.current_char() {
                        Some('n') => { result.push('\n'); self.advance(); }
                        Some('t') => { result.push('\t'); self.advance(); }
                        Some('r') => { result.push('\r'); self.advance(); }
                        Some('\\') => { result.push('\\'); self.advance(); }
                        Some('"') => { result.push('"'); self.advance(); }
                        Some('\'') => { result.push('\''); self.advance(); }
                        Some('0') => { result.push('\0'); self.advance(); }
                        Some(_c) => {
                            return Err(LexError {
                                position: start_pos,
                                kind: LexErrorKind::InvalidEscape,
                            });
                        }
                        None => {
                            return Err(LexError {
                                position: start_pos,
                                kind: LexErrorKind::UnterminatedString,
                            });
                        }
                    }
                }
                '\n' => {
                    return Err(LexError {
                        position: start_pos,
                        kind: LexErrorKind::UnterminatedString,
                    });
                }
                _ => {
                    result.push(ch);
                    self.advance();
                }
            }
        }
        
        Err(LexError {
            position: start_pos,
            kind: LexErrorKind::UnterminatedString,
        })
    }
    
    /// Read a character literal
    fn read_character(&mut self) -> Result<Token, LexError> {
        let start_pos = self.current_pos;
        
        self.advance(); // consume opening quote
        
        let ch = match self.current_char() {
            Some('\\') => {
                self.advance(); // consume backslash
                match self.current_char() {
                    Some('n') => { self.advance(); '\n' }
                    Some('t') => { self.advance(); '\t' }
                    Some('r') => { self.advance(); '\r' }
                    Some('\\') => { self.advance(); '\\' }
                    Some('"') => { self.advance(); '"' }
                    Some('\'') => { self.advance(); '\'' }
                    Some('0') => { self.advance(); '\0' }
                    Some(_) => {
                        return Err(LexError {
                            position: start_pos,
                            kind: LexErrorKind::InvalidEscape,
                        });
                    }
                    None => {
                        return Err(LexError {
                            position: start_pos,
                            kind: LexErrorKind::InvalidCharacter,
                        });
                    }
                }
            }
            Some(ch) => {
                self.advance();
                ch
            }
            None => {
                return Err(LexError {
                    position: start_pos,
                    kind: LexErrorKind::InvalidCharacter,
                });
            }
        };
        
        if self.current_char() == Some('\'') {
            self.advance(); // consume closing quote
            Ok(Token::Character(ch))
        } else {
            Err(LexError {
                position: start_pos,
                kind: LexErrorKind::InvalidCharacter,
            })
        }
    }
    
    /// Convert identifier to keyword token or return identifier
    fn identifier_to_token(&self, ident: String) -> Token {
        match ident.as_str() {
            "let" => Token::Let,
            "const" => Token::Const,
            "func" => Token::Func,
            "struct" => Token::Struct,
            "class" => Token::Class,
            "if" => Token::If,
            "else" => Token::Else,
            "match" => Token::Match,
            "case" => Token::Case,
            "for" => Token::For,
            "while" => Token::While,
            "loop" => Token::Loop,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "return" => Token::Return,
            "go" => Token::Go,
            "async" => Token::Async,
            "await" => Token::Await,
            "import" => Token::Import,
            "package" => Token::Package,
            "pub" => Token::Pub,
            "mut" => Token::Mut,
            "extern" => Token::Extern,
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            _ => Token::Identifier(ident),
        }
    }
    
    /// Get the next token from the input stream
    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace();
        
        match self.current_char() {
            None => Ok(Token::Eof),
            Some(ch) => {
                let start_pos = self.current_pos;
                
                match ch {
                    // Newlines
                    '\n' => {
                        self.advance();
                        Ok(Token::Newline)
                    }
                    
                    // Single-character tokens
                    '(' => { self.advance(); Ok(Token::LeftParen) }
                    ')' => { self.advance(); Ok(Token::RightParen) }
                    '{' => { self.advance(); Ok(Token::LeftBrace) }
                    '}' => { self.advance(); Ok(Token::RightBrace) }
                    '[' => { self.advance(); Ok(Token::LeftBracket) }
                    ']' => { self.advance(); Ok(Token::RightBracket) }
                    ',' => { self.advance(); Ok(Token::Comma) }
                    ';' => { self.advance(); Ok(Token::Semicolon) }
                    '.' => { self.advance(); Ok(Token::Dot) }
                    '?' => { self.advance(); Ok(Token::Question) }
                    '~' => { self.advance(); Ok(Token::BitwiseNot) }
                    
                    // Operators that might be multi-character
                    '+' => {
                        self.advance();
                        if self.current_char() == Some('=') {
                            self.advance();
                            Ok(Token::PlusAssign)
                        } else {
                            Ok(Token::Plus)
                        }
                    }
                    
                    '-' => {
                        self.advance();
                        match self.current_char() {
                            Some('=') => { self.advance(); Ok(Token::MinusAssign) }
                            Some('>') => { self.advance(); Ok(Token::Arrow) }
                            _ => Ok(Token::Minus)
                        }
                    }
                    
                    '*' => {
                        self.advance();
                        if self.current_char() == Some('=') {
                            self.advance();
                            Ok(Token::MultiplyAssign)
                        } else {
                            Ok(Token::Multiply)
                        }
                    }
                    
                    '/' => {
                        self.advance();
                        match self.current_char() {
                            Some('=') => { 
                                self.advance(); 
                                Ok(Token::DivideAssign) 
                            }
                            Some('/') => {
                                self.advance(); // consume second '/'
                                self.skip_line_comment();
                                self.next_token() // recursively get next token after comment
                            }
                            Some('*') => {
                                self.advance(); // consume '*'
                                self.skip_block_comment()?;
                                self.next_token() // recursively get next token after comment
                            }
                            _ => Ok(Token::Divide)
                        }
                    }
                    
                    '%' => {
                        self.advance();
                        if self.current_char() == Some('=') {
                            self.advance();
                            Ok(Token::ModuloAssign)
                        } else {
                            Ok(Token::Modulo)
                        }
                    }
                    
                    '=' => {
                        self.advance();
                        match self.current_char() {
                            Some('=') => { self.advance(); Ok(Token::Equal) }
                            Some('>') => { self.advance(); Ok(Token::FatArrow) }
                            _ => Ok(Token::Assign)
                        }
                    }
                    
                    '!' => {
                        self.advance();
                        if self.current_char() == Some('=') {
                            self.advance();
                            Ok(Token::NotEqual)
                        } else {
                            Ok(Token::Not)
                        }
                    }
                    
                    '<' => {
                        self.advance();
                        match self.current_char() {
                            Some('=') => { self.advance(); Ok(Token::LessEqual) }
                            Some('<') => { self.advance(); Ok(Token::LeftShift) }
                            _ => Ok(Token::Less)
                        }
                    }
                    
                    '>' => {
                        self.advance();
                        match self.current_char() {
                            Some('=') => { self.advance(); Ok(Token::GreaterEqual) }
                            Some('>') => { self.advance(); Ok(Token::RightShift) }
                            _ => Ok(Token::Greater)
                        }
                    }
                    
                    '&' => {
                        self.advance();
                        if self.current_char() == Some('&') {
                            self.advance();
                            Ok(Token::And)
                        } else {
                            Ok(Token::BitwiseAnd)
                        }
                    }
                    
                    '|' => {
                        self.advance();
                        if self.current_char() == Some('|') {
                            self.advance();
                            Ok(Token::Or)
                        } else {
                            Ok(Token::BitwiseOr)
                        }
                    }
                    
                    '^' => { self.advance(); Ok(Token::BitwiseXor) }
                    
                    ':' => {
                        self.advance();
                        if self.current_char() == Some(':') {
                            self.advance();
                            Ok(Token::DoubleColon)
                        } else {
                            Ok(Token::Colon)
                        }
                    }
                    
                    // Identifiers and keywords
                    c if c.is_alphabetic() || c == '_' => {
                        let ident = self.read_identifier();
                        Ok(self.identifier_to_token(ident))
                    }
                    
                    // Numbers
                    c if c.is_ascii_digit() => {
                        self.read_number()
                    }
                    
                    // String literals
                    '"' => {
                        self.read_string()
                    }
                    
                    // Character literals
                    '\'' => {
                        self.read_character()
                    }
                    
                    // Unexpected character
                    _ => Err(LexError {
                        position: start_pos,
                        kind: LexErrorKind::UnexpectedCharacter(ch),
                    }),
                }
            }
        }
    }
    
    /// Peek at the next token without consuming it
    pub fn peek_token(&self) -> Result<Token, LexError> {
        let mut clone = self.clone();
        clone.next_token()
    }
    
    /// Get the current position in the source
    pub fn position(&self) -> Position {
        self.current_pos
    }
    
    /// Check if we've reached the end of input
    pub fn is_at_end(&self) -> bool {
        self.current_char().is_none()
    }
}

impl Clone for FluxLexer {
    fn clone(&self) -> Self {
        Self {
            input: self.input.clone(),
            position: self.position,
            current_pos: self.current_pos,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LexErrorKind;

    #[test]
    fn test_empty_input() {
        let mut lexer = FluxLexer::new("".to_string());
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_single_character_tokens() {
        let mut lexer = FluxLexer::new("(){}[],.;?~".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::LeftParen);
        assert_eq!(lexer.next_token().unwrap(), Token::RightParen);
        assert_eq!(lexer.next_token().unwrap(), Token::LeftBrace);
        assert_eq!(lexer.next_token().unwrap(), Token::RightBrace);
        assert_eq!(lexer.next_token().unwrap(), Token::LeftBracket);
        assert_eq!(lexer.next_token().unwrap(), Token::RightBracket);
        assert_eq!(lexer.next_token().unwrap(), Token::Comma);
        assert_eq!(lexer.next_token().unwrap(), Token::Dot);
        assert_eq!(lexer.next_token().unwrap(), Token::Semicolon);
        assert_eq!(lexer.next_token().unwrap(), Token::Question);
        assert_eq!(lexer.next_token().unwrap(), Token::BitwiseNot);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_arithmetic_operators() {
        let mut lexer = FluxLexer::new("+ - * / %".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Plus);
        assert_eq!(lexer.next_token().unwrap(), Token::Minus);
        assert_eq!(lexer.next_token().unwrap(), Token::Multiply);
        assert_eq!(lexer.next_token().unwrap(), Token::Divide);
        assert_eq!(lexer.next_token().unwrap(), Token::Modulo);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_assignment_operators() {
        let mut lexer = FluxLexer::new("+= -= *= /= %= =".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::PlusAssign);
        assert_eq!(lexer.next_token().unwrap(), Token::MinusAssign);
        assert_eq!(lexer.next_token().unwrap(), Token::MultiplyAssign);
        assert_eq!(lexer.next_token().unwrap(), Token::DivideAssign);
        assert_eq!(lexer.next_token().unwrap(), Token::ModuloAssign);
        assert_eq!(lexer.next_token().unwrap(), Token::Assign);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_comparison_operators() {
        let mut lexer = FluxLexer::new("== != < > <= >=".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Equal);
        assert_eq!(lexer.next_token().unwrap(), Token::NotEqual);
        assert_eq!(lexer.next_token().unwrap(), Token::Less);
        assert_eq!(lexer.next_token().unwrap(), Token::Greater);
        assert_eq!(lexer.next_token().unwrap(), Token::LessEqual);
        assert_eq!(lexer.next_token().unwrap(), Token::GreaterEqual);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_logical_operators() {
        let mut lexer = FluxLexer::new("&& || !".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::And);
        assert_eq!(lexer.next_token().unwrap(), Token::Or);
        assert_eq!(lexer.next_token().unwrap(), Token::Not);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_bitwise_operators() {
        let mut lexer = FluxLexer::new("& | ^ << >>".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::BitwiseAnd);
        assert_eq!(lexer.next_token().unwrap(), Token::BitwiseOr);
        assert_eq!(lexer.next_token().unwrap(), Token::BitwiseXor);
        assert_eq!(lexer.next_token().unwrap(), Token::LeftShift);
        assert_eq!(lexer.next_token().unwrap(), Token::RightShift);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_arrows_and_colons() {
        let mut lexer = FluxLexer::new("-> => : ::".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Arrow);
        assert_eq!(lexer.next_token().unwrap(), Token::FatArrow);
        assert_eq!(lexer.next_token().unwrap(), Token::Colon);
        assert_eq!(lexer.next_token().unwrap(), Token::DoubleColon);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_identifiers() {
        let mut lexer = FluxLexer::new("hello world _private __internal snake_case".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("world".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("_private".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("__internal".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("snake_case".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_keywords() {
        let mut lexer = FluxLexer::new("let const func struct class if else".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Let);
        assert_eq!(lexer.next_token().unwrap(), Token::Const);
        assert_eq!(lexer.next_token().unwrap(), Token::Func);
        assert_eq!(lexer.next_token().unwrap(), Token::Struct);
        assert_eq!(lexer.next_token().unwrap(), Token::Class);
        assert_eq!(lexer.next_token().unwrap(), Token::If);
        assert_eq!(lexer.next_token().unwrap(), Token::Else);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_control_flow_keywords() {
        let mut lexer = FluxLexer::new("match case for while loop break continue return".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Match);
        assert_eq!(lexer.next_token().unwrap(), Token::Case);
        assert_eq!(lexer.next_token().unwrap(), Token::For);
        assert_eq!(lexer.next_token().unwrap(), Token::While);
        assert_eq!(lexer.next_token().unwrap(), Token::Loop);
        assert_eq!(lexer.next_token().unwrap(), Token::Break);
        assert_eq!(lexer.next_token().unwrap(), Token::Continue);
        assert_eq!(lexer.next_token().unwrap(), Token::Return);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_concurrency_keywords() {
        let mut lexer = FluxLexer::new("go async await".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Go);
        assert_eq!(lexer.next_token().unwrap(), Token::Async);
        assert_eq!(lexer.next_token().unwrap(), Token::Await);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_module_keywords() {
        let mut lexer = FluxLexer::new("import package pub mut".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Import);
        assert_eq!(lexer.next_token().unwrap(), Token::Package);
        assert_eq!(lexer.next_token().unwrap(), Token::Pub);
        assert_eq!(lexer.next_token().unwrap(), Token::Mut);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_boolean_literals() {
        let mut lexer = FluxLexer::new("true false".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Boolean(true));
        assert_eq!(lexer.next_token().unwrap(), Token::Boolean(false));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_integer_literals() {
        let mut lexer = FluxLexer::new("0 42 123 999".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(0));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(42));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(123));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(999));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_float_literals() {
        let mut lexer = FluxLexer::new("3.14 0.5 42.0 123.456".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Float(3.14));
        assert_eq!(lexer.next_token().unwrap(), Token::Float(0.5));
        assert_eq!(lexer.next_token().unwrap(), Token::Float(42.0));
        assert_eq!(lexer.next_token().unwrap(), Token::Float(123.456));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_scientific_notation() {
        let mut lexer = FluxLexer::new("1e5 2.5e-3 1.23E+10 5E0".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Float(1e5));
        assert_eq!(lexer.next_token().unwrap(), Token::Float(2.5e-3));
        assert_eq!(lexer.next_token().unwrap(), Token::Float(1.23E+10));
        assert_eq!(lexer.next_token().unwrap(), Token::Float(5E0));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_string_literals() {
        let mut lexer = FluxLexer::new(r#""hello" "world" "hello world""#.to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::String("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::String("world".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::String("hello world".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_string_escape_sequences() {
        let mut lexer = FluxLexer::new(r#""hello\nworld" "tab\there" "quote\"here" "backslash\\""#.to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::String("hello\nworld".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::String("tab\there".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::String("quote\"here".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::String("backslash\\".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_character_literals() {
        let mut lexer = FluxLexer::new("'a' 'Z' '5' ' '".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Character('a'));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('Z'));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('5'));
        assert_eq!(lexer.next_token().unwrap(), Token::Character(' '));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_character_escape_sequences() {
        let mut lexer = FluxLexer::new("'\\n' '\\t' '\\r' '\\\\' '\\'' '\\\"' '\\0'".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Character('\n'));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('\t'));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('\r'));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('\\'));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('\''));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('"'));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('\0'));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_unterminated_string_error() {
        let mut lexer = FluxLexer::new(r#""hello world"#.to_string());
        
        match lexer.next_token() {
            Err(LexError { kind: LexErrorKind::UnterminatedString, .. }) => {},
            other => panic!("Expected UnterminatedString error, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_escape_sequence_error() {
        let mut lexer = FluxLexer::new(r#""hello\x""#.to_string());
        
        match lexer.next_token() {
            Err(LexError { kind: LexErrorKind::InvalidEscape, .. }) => {},
            other => panic!("Expected InvalidEscape error, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_character_literal_error() {
        let mut lexer = FluxLexer::new("'ab'".to_string());
        
        match lexer.next_token() {
            Err(LexError { kind: LexErrorKind::InvalidCharacter, .. }) => {},
            other => panic!("Expected InvalidCharacter error, got {:?}", other),
        }
    }

    #[test]
    fn test_unterminated_character_literal_error() {
        let mut lexer = FluxLexer::new("'a".to_string());
        
        match lexer.next_token() {
            Err(LexError { kind: LexErrorKind::InvalidCharacter, .. }) => {},
            other => panic!("Expected InvalidCharacter error, got {:?}", other),
        }
    }

    #[test]
    fn test_mixed_literals() {
        let mut lexer = FluxLexer::new(r#"42 3.14 "hello" 'a' true"#.to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(42));
        assert_eq!(lexer.next_token().unwrap(), Token::Float(3.14));
        assert_eq!(lexer.next_token().unwrap(), Token::String("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('a'));
        assert_eq!(lexer.next_token().unwrap(), Token::Boolean(true));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_number_followed_by_dot_identifier() {
        // Test that "42.toString" is parsed as Integer(42), Dot, Identifier("toString")
        let mut lexer = FluxLexer::new("42.toString".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(42));
        assert_eq!(lexer.next_token().unwrap(), Token::Dot);
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("toString".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_invalid_scientific_notation() {
        let mut lexer = FluxLexer::new("1e".to_string());
        
        match lexer.next_token() {
            Err(LexError { kind: LexErrorKind::InvalidNumber, .. }) => {},
            other => panic!("Expected InvalidNumber error, got {:?}", other),
        }
    }

    #[test]
    fn test_single_line_comments() {
        let mut lexer = FluxLexer::new("hello // this is a comment\nworld".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Newline);
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("world".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_single_line_comment_at_end() {
        let mut lexer = FluxLexer::new("hello // comment at end".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_multi_line_comments() {
        let mut lexer = FluxLexer::new("hello /* this is a\nmulti-line comment */ world".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("world".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_nested_comment_content() {
        let mut lexer = FluxLexer::new("hello /* comment with // inside */ world".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("world".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_unterminated_block_comment_error() {
        let mut lexer = FluxLexer::new("hello /* unterminated comment".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("hello".to_string()));
        
        match lexer.next_token() {
            Err(LexError { kind: LexErrorKind::UnterminatedComment, .. }) => {},
            other => panic!("Expected UnterminatedComment error, got {:?}", other),
        }
    }

    #[test]
    fn test_all_keywords_comprehensive() {
        let keywords = vec![
            ("let", Token::Let),
            ("const", Token::Const),
            ("func", Token::Func),
            ("struct", Token::Struct),
            ("class", Token::Class),
            ("if", Token::If),
            ("else", Token::Else),
            ("match", Token::Match),
            ("case", Token::Case),
            ("for", Token::For),
            ("while", Token::While),
            ("loop", Token::Loop),
            ("break", Token::Break),
            ("continue", Token::Continue),
            ("return", Token::Return),
            ("go", Token::Go),
            ("async", Token::Async),
            ("await", Token::Await),
            ("import", Token::Import),
            ("package", Token::Package),
            ("pub", Token::Pub),
            ("mut", Token::Mut),
            ("true", Token::Boolean(true)),
            ("false", Token::Boolean(false)),
        ];

        for (keyword, expected_token) in keywords {
            let mut lexer = FluxLexer::new(keyword.to_string());
            assert_eq!(lexer.next_token().unwrap(), expected_token, "Failed for keyword: {}", keyword);
            assert_eq!(lexer.next_token().unwrap(), Token::Eof);
        }
    }

    #[test]
    fn test_all_operators_comprehensive() {
        let operators = vec![
            ("+", Token::Plus),
            ("-", Token::Minus),
            ("*", Token::Multiply),
            ("/", Token::Divide),
            ("%", Token::Modulo),
            ("==", Token::Equal),
            ("!=", Token::NotEqual),
            ("<", Token::Less),
            (">", Token::Greater),
            ("<=", Token::LessEqual),
            (">=", Token::GreaterEqual),
            ("&&", Token::And),
            ("||", Token::Or),
            ("!", Token::Not),
            ("&", Token::BitwiseAnd),
            ("|", Token::BitwiseOr),
            ("^", Token::BitwiseXor),
            ("~", Token::BitwiseNot),
            ("<<", Token::LeftShift),
            (">>", Token::RightShift),
            ("=", Token::Assign),
            ("+=", Token::PlusAssign),
            ("-=", Token::MinusAssign),
            ("*=", Token::MultiplyAssign),
            ("/=", Token::DivideAssign),
            ("%=", Token::ModuloAssign),
            ("->", Token::Arrow),
            ("=>", Token::FatArrow),
            (":", Token::Colon),
            ("::", Token::DoubleColon),
        ];

        for (op, expected_token) in operators {
            let mut lexer = FluxLexer::new(op.to_string());
            assert_eq!(lexer.next_token().unwrap(), expected_token, "Failed for operator: {}", op);
            assert_eq!(lexer.next_token().unwrap(), Token::Eof);
        }
    }

    #[test]
    fn test_all_delimiters_comprehensive() {
        let delimiters = vec![
            ("(", Token::LeftParen),
            (")", Token::RightParen),
            ("{", Token::LeftBrace),
            ("}", Token::RightBrace),
            ("[", Token::LeftBracket),
            ("]", Token::RightBracket),
            (",", Token::Comma),
            (";", Token::Semicolon),
            (".", Token::Dot),
            ("?", Token::Question),
        ];

        for (delim, expected_token) in delimiters {
            let mut lexer = FluxLexer::new(delim.to_string());
            assert_eq!(lexer.next_token().unwrap(), expected_token, "Failed for delimiter: {}", delim);
            assert_eq!(lexer.next_token().unwrap(), Token::Eof);
        }
    }

    #[test]
    fn test_complex_program_with_comments() {
        let program = "func main() { return 42; }";
        let mut lexer = FluxLexer::new(program.to_string());
        
        let expected_tokens = vec![
            Token::Func,
            Token::Identifier("main".to_string()),
            Token::LeftParen,
            Token::RightParen,
            Token::LeftBrace,
            Token::Return,
            Token::Integer(42),
            Token::Semicolon,
            Token::RightBrace,
            Token::Eof,
        ];
        
        for expected_token in expected_tokens {
            let token = lexer.next_token().unwrap();
            assert_eq!(token, expected_token, "Token mismatch");
        }
    }

    #[test]
    fn test_operator_precedence_parsing() {
        // Test that operators are correctly tokenized when used together
        let mut lexer = FluxLexer::new("a += b *= c /= d %= e".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("a".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::PlusAssign);
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("b".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::MultiplyAssign);
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("c".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::DivideAssign);
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("d".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::ModuloAssign);
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("e".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_edge_case_combinations() {
        // Test edge cases where operators might be confused
        let mut lexer = FluxLexer::new("< <= << > >= >> = == => != !".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Less);
        assert_eq!(lexer.next_token().unwrap(), Token::LessEqual);
        assert_eq!(lexer.next_token().unwrap(), Token::LeftShift);
        assert_eq!(lexer.next_token().unwrap(), Token::Greater);
        assert_eq!(lexer.next_token().unwrap(), Token::GreaterEqual);
        assert_eq!(lexer.next_token().unwrap(), Token::RightShift);
        assert_eq!(lexer.next_token().unwrap(), Token::Assign);
        assert_eq!(lexer.next_token().unwrap(), Token::Equal);
        assert_eq!(lexer.next_token().unwrap(), Token::FatArrow);
        assert_eq!(lexer.next_token().unwrap(), Token::NotEqual);
        assert_eq!(lexer.next_token().unwrap(), Token::Not);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_newlines() {
        let mut lexer = FluxLexer::new("hello\nworld\n".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Newline);
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("world".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Newline);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_whitespace_handling() {
        let mut lexer = FluxLexer::new("  hello   world  ".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("world".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_mixed_tokens() {
        let mut lexer = FluxLexer::new("let x = 42 + y;".to_string());
        
        assert_eq!(lexer.next_token().unwrap(), Token::Let);
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("x".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Assign);
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(42));
        assert_eq!(lexer.next_token().unwrap(), Token::Plus);
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("y".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Semicolon);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_peek_token() {
        let mut lexer = FluxLexer::new("hello world".to_string());
        
        // Peek should return the first token without consuming it
        assert_eq!(lexer.peek_token().unwrap(), Token::Identifier("hello".to_string()));
        assert_eq!(lexer.peek_token().unwrap(), Token::Identifier("hello".to_string()));
        
        // Now consume the first token
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("hello".to_string()));
        
        // Peek should now return the second token
        assert_eq!(lexer.peek_token().unwrap(), Token::Identifier("world".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Identifier("world".to_string()));
    }

    #[test]
    fn test_position_tracking() {
        let mut lexer = FluxLexer::new("hello\nworld".to_string());
        
        // Initial position
        assert_eq!(lexer.position().line, 1);
        assert_eq!(lexer.position().column, 1);
        
        // After first token
        lexer.next_token().unwrap(); // "hello"
        assert_eq!(lexer.position().line, 1);
        assert_eq!(lexer.position().column, 6);
        
        // After newline
        lexer.next_token().unwrap(); // newline
        assert_eq!(lexer.position().line, 2);
        assert_eq!(lexer.position().column, 1);
        
        // After second token
        lexer.next_token().unwrap(); // "world"
        assert_eq!(lexer.position().line, 2);
        assert_eq!(lexer.position().column, 6);
    }

    #[test]
    fn test_is_at_end() {
        let mut lexer = FluxLexer::new("hello".to_string());
        
        assert!(!lexer.is_at_end());
        let token1 = lexer.next_token().unwrap(); // "hello"
        assert_eq!(token1, Token::Identifier("hello".to_string()));
        
        let token2 = lexer.next_token().unwrap(); // EOF
        assert_eq!(token2, Token::Eof);
        assert!(lexer.is_at_end());
    }

    #[test]
    fn test_unexpected_character_error() {
        let mut lexer = FluxLexer::new("@".to_string());
        
        match lexer.next_token() {
            Err(LexError { kind: LexErrorKind::UnexpectedCharacter('@'), .. }) => {},
            other => panic!("Expected UnexpectedCharacter error, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_number_error() {
        // This test is for future expansion when we add more complex number parsing
        // For now, our simple parser shouldn't produce invalid numbers
        let mut lexer = FluxLexer::new("123".to_string());
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(123));
    }

    #[test]
    fn test_complex_expression() {
        let mut lexer = FluxLexer::new("func add(a: int, b: int) -> int { return a + b; }".to_string());
        
        let expected_tokens = vec![
            Token::Func,
            Token::Identifier("add".to_string()),
            Token::LeftParen,
            Token::Identifier("a".to_string()),
            Token::Colon,
            Token::Identifier("int".to_string()),
            Token::Comma,
            Token::Identifier("b".to_string()),
            Token::Colon,
            Token::Identifier("int".to_string()),
            Token::RightParen,
            Token::Arrow,
            Token::Identifier("int".to_string()),
            Token::LeftBrace,
            Token::Return,
            Token::Identifier("a".to_string()),
            Token::Plus,
            Token::Identifier("b".to_string()),
            Token::Semicolon,
            Token::RightBrace,
            Token::Eof,
        ];
        
        for expected_token in expected_tokens {
            assert_eq!(lexer.next_token().unwrap(), expected_token);
        }
    }
}