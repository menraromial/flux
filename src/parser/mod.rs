//! Parser module for the Flux language
//! 
//! Provides parsing of token streams into Abstract Syntax Trees (AST).

use crate::error::{ParseError, ParseErrorKind};
use crate::lexer::{FluxLexer, Token};
use crate::position::Span;

pub mod ast;

pub use ast::*;

/// Core parser trait for building ASTs from token streams
pub trait Parser {
    /// Parse a complete program from the token stream
    fn parse_program(&mut self) -> Result<Program, ParseError>;
    
    /// Parse a single expression
    fn parse_expression(&mut self) -> Result<Expression, ParseError>;
    
    /// Parse a single statement
    fn parse_statement(&mut self) -> Result<Statement, ParseError>;
    
    /// Parse a function declaration
    fn parse_function(&mut self) -> Result<Function, ParseError>;
    
    /// Parse a struct declaration
    fn parse_struct(&mut self) -> Result<Struct, ParseError>;
    
    /// Check if we're at the end of input
    fn is_at_end(&self) -> bool;
}

/// Default implementation of the Flux parser using recursive descent
pub struct FluxParser {
    lexer: FluxLexer,
    current_token: Token,
    peek_token: Token,
}

impl FluxParser {
    /// Create a new parser with the given lexer
    pub fn new(mut lexer: FluxLexer) -> Result<Self, ParseError> {
        let current_token = lexer.next_token().map_err(|e| ParseError {
            span: Span::single(e.position),
            kind: ParseErrorKind::InvalidSyntax { 
                message: format!("Lexical error: {}", e) 
            },
        })?;
        
        let peek_token = lexer.next_token().map_err(|e| ParseError {
            span: Span::single(e.position),
            kind: ParseErrorKind::InvalidSyntax { 
                message: format!("Lexical error: {}", e) 
            },
        })?;
        
        Ok(Self {
            lexer,
            current_token,
            peek_token,
        })
    }
    
    /// Advance to the next token
    fn advance(&mut self) -> Result<(), ParseError> {
        self.current_token = std::mem::replace(&mut self.peek_token, Token::Eof);
        self.peek_token = self.lexer.next_token().map_err(|e| ParseError {
            span: Span::single(e.position),
            kind: ParseErrorKind::InvalidSyntax { 
                message: format!("Lexical error: {}", e) 
            },
        })?;
        Ok(())
    }
    
    /// Check if current token matches the expected token
    fn check(&self, token: &Token) -> bool {
        std::mem::discriminant(&self.current_token) == std::mem::discriminant(token)
    }
    
    /// Consume a token if it matches, otherwise return an error
    fn consume(&mut self, expected: Token, message: &str) -> Result<(), ParseError> {
        if self.check(&expected) {
            self.advance()?;
            Ok(())
        } else {
            Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: format!("{} ({})", expected, message),
                    found: format!("{}", self.current_token),
                },
            })
        }
    }

    /// Synchronize the parser after an error by skipping tokens until we reach a statement boundary
    fn synchronize(&mut self) -> Result<(), ParseError> {
        self.advance()?;

        while !self.is_at_end() {
            // If we see a semicolon, we're at the end of a statement
            if matches!(self.current_token, Token::Semicolon) {
                self.advance()?;
                return Ok(());
            }

            // Look for statement keywords that indicate the start of a new statement
            match &self.current_token {
                Token::Let | Token::Const | Token::Func | Token::Struct | Token::Class |
                Token::If | Token::While | Token::For | Token::Match | Token::Return |
                Token::Break | Token::Continue | Token::Go => {
                    return Ok(());
                }
                _ => {
                    self.advance()?;
                }
            }
        }

        Ok(())
    }

    /// Try to recover from a parsing error and continue parsing
    fn recover_from_error(&mut self, error: ParseError) -> ParseError {
        // Log the error (in a real implementation, you might want to collect multiple errors)
        eprintln!("Parse error: {:?}", error);
        
        // Try to synchronize to the next statement
        if let Err(sync_error) = self.synchronize() {
            // If synchronization fails, return the original error
            error
        } else {
            // Return the original error but continue parsing
            error
        }
    }

    /// Parse with error recovery - continues parsing even after errors
    fn parse_with_recovery<T, F>(&mut self, parse_fn: F, context: &str) -> Result<T, ParseError>
    where
        F: FnOnce(&mut Self) -> Result<T, ParseError>,
    {
        match parse_fn(self) {
            Ok(result) => Ok(result),
            Err(error) => {
                // Enhance error message with context
                let enhanced_error = ParseError {
                    span: error.span,
                    kind: match error.kind {
                        ParseErrorKind::UnexpectedToken { expected, found } => {
                            ParseErrorKind::UnexpectedToken {
                                expected: format!("{} (while parsing {})", expected, context),
                                found,
                            }
                        }
                        ParseErrorKind::InvalidExpression => {
                            ParseErrorKind::InvalidExpression
                        }
                        ParseErrorKind::InvalidSyntax { message } => {
                            ParseErrorKind::InvalidSyntax {
                                message: format!("{} (while parsing {})", message, context),
                            }
                        }
                        ParseErrorKind::UnexpectedEof => ParseErrorKind::UnexpectedEof,
                        ParseErrorKind::MissingSemicolon => ParseErrorKind::MissingSemicolon,
                        ParseErrorKind::InvalidStatement => ParseErrorKind::InvalidStatement,
                    },
                };
                
                Err(self.recover_from_error(enhanced_error))
            }
        }
    }

    // Implementation methods (not part of trait)
    fn parse_program_impl(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        let mut errors = Vec::new();
        
        while !self.is_at_end() {
            match &self.current_token {
                Token::Pub => {
                    // Look ahead to see what kind of declaration this is
                    match &self.peek_token {
                        Token::Func => {
                            match self.parse_with_recovery(|p| p.parse_function_impl(), "function declaration") {
                                Ok(func) => items.push(Item::Function(func)),
                                Err(error) => {
                                    errors.push(error);
                                    // Continue parsing after error recovery
                                }
                            }
                        }
                        Token::Struct => {
                            match self.parse_with_recovery(|p| p.parse_struct_impl(), "struct declaration") {
                                Ok(struct_) => items.push(Item::Struct(struct_)),
                                Err(error) => {
                                    errors.push(error);
                                }
                            }
                        }
                        Token::Extern => {
                            match self.parse_with_recovery(|p| p.parse_extern_function_impl(), "extern function declaration") {
                                Ok(extern_func) => items.push(Item::ExternFunction(extern_func)),
                                Err(error) => {
                                    errors.push(error);
                                }
                            }
                        }
                        _ => {
                            let error = ParseError {
                                span: Span::single(self.lexer.position()),
                                kind: ParseErrorKind::UnexpectedToken {
                                    expected: "function or struct declaration after 'pub'".to_string(),
                                    found: format!("{}", self.peek_token),
                                },
                            };
                            errors.push(self.recover_from_error(error));
                        }
                    }
                }
                Token::Func => {
                    match self.parse_with_recovery(|p| p.parse_function_impl(), "function declaration") {
                        Ok(func) => items.push(Item::Function(func)),
                        Err(error) => {
                            errors.push(error);
                        }
                    }
                }
                Token::Struct => {
                    match self.parse_with_recovery(|p| p.parse_struct_impl(), "struct declaration") {
                        Ok(struct_) => items.push(Item::Struct(struct_)),
                        Err(error) => {
                            errors.push(error);
                        }
                    }
                }
                Token::Const => {
                    // Parse top-level const declaration
                    match self.parse_with_recovery(|p| p.parse_const_declaration(), "const declaration") {
                        Ok(const_) => items.push(Item::Const(const_)),
                        Err(error) => {
                            errors.push(error);
                        }
                    }
                }
                Token::Extern => {
                    match self.parse_with_recovery(|p| p.parse_extern_function_impl(), "extern function declaration") {
                        Ok(extern_func) => items.push(Item::ExternFunction(extern_func)),
                        Err(error) => {
                            errors.push(error);
                        }
                    }
                }
                Token::Eof => break,
                _ => {
                    let error = ParseError {
                        span: Span::single(self.lexer.position()),
                        kind: ParseErrorKind::UnexpectedToken {
                            expected: "function, struct, or const declaration".to_string(),
                            found: format!("{}", self.current_token),
                        },
                    };
                    errors.push(self.recover_from_error(error));
                }
            }
        }
        
        // For now, return the first error if any occurred
        // In a more sophisticated implementation, you might collect all errors
        if let Some(first_error) = errors.into_iter().next() {
            return Err(first_error);
        }
        
        Ok(Program {
            package: "main".to_string(), // Default package
            imports: Vec::new(),         // No imports for now
            items,
        })
    }

    fn parse_const_declaration(&mut self) -> Result<Const, ParseError> {
        // Check for visibility modifier
        let visibility = if matches!(self.current_token, Token::Pub) {
            self.advance()?; // consume 'pub'
            Visibility::Public
        } else {
            Visibility::Private
        };

        self.consume(Token::Const, "Expected 'const'")?;
        
        if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance()?;
            
            self.consume(Token::Colon, "Expected ':' after const name")?;
            let type_annotation = self.parse_type()?;
            
            self.consume(Token::Assign, "Expected '=' after const type")?;
            let value = self.parse_expression_impl()?;
            
            Ok(Const {
                name,
                type_: type_annotation,
                value,
                visibility,
            })
        } else {
            Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "identifier".to_string(),
                    found: format!("{}", self.current_token),
                },
            })
        }
    }
    
    fn parse_expression_impl(&mut self) -> Result<Expression, ParseError> {
        self.parse_logical_or()
    }

    // Logical OR (lowest precedence)
    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_logical_and()?;

        while matches!(self.current_token, Token::Or) {
            let op = BinaryOp::Or;
            self.advance()?;
            let right = self.parse_logical_and()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Logical AND
    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_equality()?;

        while matches!(self.current_token, Token::And) {
            let op = BinaryOp::And;
            self.advance()?;
            let right = self.parse_equality()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Equality operators
    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_comparison()?;

        while let Some(op) = match &self.current_token {
            Token::Equal => Some(BinaryOp::Equal),
            Token::NotEqual => Some(BinaryOp::NotEqual),
            _ => None,
        } {
            self.advance()?;
            let right = self.parse_comparison()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Comparison operators
    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bitwise_or()?;

        while let Some(op) = match &self.current_token {
            Token::Greater => Some(BinaryOp::Greater),
            Token::GreaterEqual => Some(BinaryOp::GreaterEqual),
            Token::Less => Some(BinaryOp::Less),
            Token::LessEqual => Some(BinaryOp::LessEqual),
            _ => None,
        } {
            self.advance()?;
            let right = self.parse_bitwise_or()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Bitwise OR
    fn parse_bitwise_or(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bitwise_xor()?;

        while matches!(self.current_token, Token::BitwiseOr) {
            let op = BinaryOp::BitwiseOr;
            self.advance()?;
            let right = self.parse_bitwise_xor()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Bitwise XOR
    fn parse_bitwise_xor(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bitwise_and()?;

        while matches!(self.current_token, Token::BitwiseXor) {
            let op = BinaryOp::BitwiseXor;
            self.advance()?;
            let right = self.parse_bitwise_and()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Bitwise AND
    fn parse_bitwise_and(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_shift()?;

        while matches!(self.current_token, Token::BitwiseAnd) {
            let op = BinaryOp::BitwiseAnd;
            self.advance()?;
            let right = self.parse_shift()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Shift operators
    fn parse_shift(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_term()?;

        while let Some(op) = match &self.current_token {
            Token::LeftShift => Some(BinaryOp::LeftShift),
            Token::RightShift => Some(BinaryOp::RightShift),
            _ => None,
        } {
            self.advance()?;
            let right = self.parse_term()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Addition and subtraction
    fn parse_term(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_factor()?;

        while let Some(op) = match &self.current_token {
            Token::Plus => Some(BinaryOp::Add),
            Token::Minus => Some(BinaryOp::Subtract),
            _ => None,
        } {
            self.advance()?;
            let right = self.parse_factor()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Multiplication, division, and modulo
    fn parse_factor(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_unary()?;

        while let Some(op) = match &self.current_token {
            Token::Multiply => Some(BinaryOp::Multiply),
            Token::Divide => Some(BinaryOp::Divide),
            Token::Modulo => Some(BinaryOp::Modulo),
            _ => None,
        } {
            self.advance()?;
            let right = self.parse_unary()?;
            expr = Expression::Binary(Box::new(expr), op, Box::new(right));
        }

        Ok(expr)
    }

    // Unary operators
    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if let Some(op) = match &self.current_token {
            Token::Not => Some(UnaryOp::Not),
            Token::Minus => Some(UnaryOp::Minus),
            Token::Plus => Some(UnaryOp::Plus),
            Token::BitwiseNot => Some(UnaryOp::BitwiseNot),
            _ => None,
        } {
            self.advance()?;
            let expr = self.parse_unary()?;
            Ok(Expression::Unary(op, Box::new(expr)))
        } else {
            self.parse_postfix()
        }
    }

    // Postfix expressions (calls, indexing, field access)
    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            match &self.current_token {
                Token::LeftParen => {
                    // Function call
                    self.advance()?; // consume '('
                    let mut args = Vec::new();
                    
                    if !matches!(self.current_token, Token::RightParen) {
                        loop {
                            args.push(self.parse_expression_impl()?);
                            if matches!(self.current_token, Token::Comma) {
                                self.advance()?; // consume ','
                            } else {
                                break;
                            }
                        }
                    }
                    
                    self.consume(Token::RightParen, "Expected ')' after function arguments")?;
                    expr = Expression::Call(Box::new(expr), args);
                }
                Token::LeftBracket => {
                    // Array/map indexing
                    self.advance()?; // consume '['
                    let index = self.parse_expression_impl()?;
                    self.consume(Token::RightBracket, "Expected ']' after index")?;
                    expr = Expression::Index(Box::new(expr), Box::new(index));
                }
                Token::Dot => {
                    // Field access
                    self.advance()?; // consume '.'
                    if let Token::Identifier(field_name) = &self.current_token {
                        let field_name = field_name.clone();
                        self.advance()?;
                        expr = Expression::Field(Box::new(expr), field_name);
                    } else {
                        return Err(ParseError {
                            span: Span::single(self.lexer.position()),
                            kind: ParseErrorKind::UnexpectedToken {
                                expected: "field name".to_string(),
                                found: format!("{}", self.current_token),
                            },
                        });
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    // Primary expressions (literals, identifiers, parenthesized expressions, etc.)
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match &self.current_token {
            // Literals
            Token::Integer(n) => {
                let value = *n;
                self.advance()?;
                Ok(Expression::Literal(Literal::Integer(value)))
            }
            Token::Float(f) => {
                let value = *f;
                self.advance()?;
                Ok(Expression::Literal(Literal::Float(value)))
            }
            Token::String(s) => {
                let value = s.clone();
                self.advance()?;
                Ok(Expression::Literal(Literal::String(value)))
            }
            Token::Boolean(b) => {
                let value = *b;
                self.advance()?;
                Ok(Expression::Literal(Literal::Boolean(value)))
            }
            Token::Character(c) => {
                let value = *c;
                self.advance()?;
                Ok(Expression::Literal(Literal::Character(value)))
            }
            
            // Identifier
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance()?;
                Ok(Expression::Identifier(name))
            }
            
            // Parenthesized expression
            Token::LeftParen => {
                self.advance()?; // consume '('
                let expr = self.parse_expression_impl()?;
                self.consume(Token::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            
            // Array literal
            Token::LeftBracket => {
                self.advance()?; // consume '['
                let mut elements = Vec::new();
                
                if !matches!(self.current_token, Token::RightBracket) {
                    loop {
                        elements.push(self.parse_expression_impl()?);
                        if matches!(self.current_token, Token::Comma) {
                            self.advance()?; // consume ','
                        } else {
                            break;
                        }
                    }
                }
                
                self.consume(Token::RightBracket, "Expected ']' after array elements")?;
                Ok(Expression::Array(elements))
            }
            
            // Block expression (simplified - no map literals for now)
            Token::LeftBrace => {
                self.parse_block_expression()
            }
            
            _ => Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::InvalidExpression,
            }),
        }
    }

    // Parse a block expression
    fn parse_block_expression(&mut self) -> Result<Expression, ParseError> {
        self.advance()?; // consume '{'
        
        let mut statements = Vec::new();
        while !matches!(self.current_token, Token::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement_impl()?);
        }
        
        self.consume(Token::RightBrace, "Expected '}' after block")?;
        Ok(Expression::Block(Block { statements }))
    }
    
    fn parse_statement_impl(&mut self) -> Result<Statement, ParseError> {
        match &self.current_token {
            Token::Let => {
                self.parse_let_statement()
            }
            Token::Const => {
                self.parse_const_statement()
            }
            Token::Return => {
                self.parse_return_statement()
            }
            Token::Break => {
                self.parse_break_statement()
            }
            Token::Continue => {
                self.parse_continue_statement()
            }
            Token::Go => {
                self.parse_go_statement()
            }
            Token::If => {
                self.parse_if_statement()
            }
            Token::While => {
                self.parse_while_statement()
            }
            Token::For => {
                self.parse_for_statement()
            }
            Token::Match => {
                self.parse_match_statement()
            }
            _ => {
                // Check if this might be an assignment
                if let Token::Identifier(_) = &self.current_token {
                    // Look ahead to see if this is an assignment
                    if matches!(self.peek_token, Token::Assign | Token::PlusAssign | Token::MinusAssign | 
                               Token::MultiplyAssign | Token::DivideAssign | Token::ModuloAssign) {
                        self.parse_assignment_statement()
                    } else {
                        // Parse as expression statement
                        let expr = self.parse_expression_impl()?;
                        Ok(Statement::Expression(expr))
                    }
                } else {
                    // Parse as expression statement
                    let expr = self.parse_expression_impl()?;
                    Ok(Statement::Expression(expr))
                }
            }
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'let'
        
        if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance()?;
            
            // Optional type annotation
            let type_annotation = if matches!(self.current_token, Token::Colon) {
                self.advance()?; // consume ':'
                Some(self.parse_type()?)
            } else {
                None
            };
            
            // Optional initialization
            let value = if matches!(self.current_token, Token::Assign) {
                self.advance()?; // consume '='
                Some(self.parse_expression_impl()?)
            } else {
                None
            };
            
            Ok(Statement::Let(name, type_annotation, value))
        } else {
            Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "identifier".to_string(),
                    found: format!("{}", self.current_token),
                },
            })
        }
    }

    fn parse_const_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'const'
        
        if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance()?;
            
            self.consume(Token::Colon, "Expected ':' after const name")?;
            let type_annotation = self.parse_type()?;
            
            self.consume(Token::Assign, "Expected '=' after const type")?;
            let value = self.parse_expression_impl()?;
            
            Ok(Statement::Const(name, type_annotation, value))
        } else {
            Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "identifier".to_string(),
                    found: format!("{}", self.current_token),
                },
            })
        }
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'return'
        
        if self.check(&Token::Semicolon) || self.check(&Token::RightBrace) || self.is_at_end() {
            Ok(Statement::Return(None))
        } else {
            let expr = self.parse_expression_impl()?;
            Ok(Statement::Return(Some(expr)))
        }
    }

    fn parse_break_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'break'
        
        if self.check(&Token::Semicolon) || self.check(&Token::RightBrace) || self.is_at_end() {
            Ok(Statement::Break(None))
        } else {
            let expr = self.parse_expression_impl()?;
            Ok(Statement::Break(Some(expr)))
        }
    }

    fn parse_continue_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'continue'
        Ok(Statement::Continue)
    }

    fn parse_go_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'go'
        let expr = self.parse_expression_impl()?;
        Ok(Statement::Go(expr))
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'if'
        let condition = self.parse_expression_impl()?;
        let then_block = self.parse_block()?;
        
        let else_block = if matches!(self.current_token, Token::Else) {
            self.advance()?; // consume 'else'
            Some(self.parse_block()?)
        } else {
            None
        };
        
        Ok(Statement::If(condition, then_block, else_block))
    }

    fn parse_while_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'while'
        let condition = self.parse_expression_impl()?;
        let body = self.parse_block()?;
        Ok(Statement::While(condition, body))
    }

    fn parse_for_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'for'
        
        if let Token::Identifier(var_name) = &self.current_token {
            let var_name = var_name.clone();
            self.advance()?;
            
            // Expect 'in' keyword (we'll use identifier for now)
            if let Token::Identifier(keyword) = &self.current_token {
                if keyword == "in" {
                    self.advance()?;
                    let iterable = self.parse_expression_impl()?;
                    let body = self.parse_block()?;
                    Ok(Statement::For(var_name, iterable, body))
                } else {
                    Err(ParseError {
                        span: Span::single(self.lexer.position()),
                        kind: ParseErrorKind::UnexpectedToken {
                            expected: "'in'".to_string(),
                            found: format!("{}", self.current_token),
                        },
                    })
                }
            } else {
                Err(ParseError {
                    span: Span::single(self.lexer.position()),
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "'in'".to_string(),
                        found: format!("{}", self.current_token),
                    },
                })
            }
        } else {
            Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "variable name".to_string(),
                    found: format!("{}", self.current_token),
                },
            })
        }
    }

    fn parse_match_statement(&mut self) -> Result<Statement, ParseError> {
        self.advance()?; // consume 'match'
        let expr = self.parse_expression_impl()?;
        self.consume(Token::LeftBrace, "Expected '{' after match expression")?;
        
        let mut arms = Vec::new();
        while !matches!(self.current_token, Token::RightBrace) && !self.is_at_end() {
            arms.push(self.parse_match_arm()?);
        }
        
        self.consume(Token::RightBrace, "Expected '}' after match arms")?;
        Ok(Statement::Match(expr, arms))
    }

    fn parse_assignment_statement(&mut self) -> Result<Statement, ParseError> {
        let target = self.parse_expression_impl()?;
        
        // For now, only handle simple assignment
        self.consume(Token::Assign, "Expected '=' in assignment")?;
        let value = self.parse_expression_impl()?;
        
        Ok(Statement::Assignment(target, value))
    }

    // Parse a match arm
    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let pattern = self.parse_pattern()?;
        
        let guard = if matches!(self.current_token, Token::If) {
            self.advance()?; // consume 'if'
            Some(self.parse_expression_impl()?)
        } else {
            None
        };
        
        self.consume(Token::FatArrow, "Expected '=>' after match pattern")?;
        let body = self.parse_block()?;
        
        Ok(MatchArm { pattern, guard, body })
    }

    // Parse a pattern
    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        match &self.current_token {
            Token::Integer(n) => {
                let value = *n;
                self.advance()?;
                Ok(Pattern::Literal(Literal::Integer(value)))
            }
            Token::Float(f) => {
                let value = *f;
                self.advance()?;
                Ok(Pattern::Literal(Literal::Float(value)))
            }
            Token::String(s) => {
                let value = s.clone();
                self.advance()?;
                Ok(Pattern::Literal(Literal::String(value)))
            }
            Token::Boolean(b) => {
                let value = *b;
                self.advance()?;
                Ok(Pattern::Literal(Literal::Boolean(value)))
            }
            Token::Character(c) => {
                let value = *c;
                self.advance()?;
                Ok(Pattern::Literal(Literal::Character(value)))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance()?;
                if name == "_" {
                    Ok(Pattern::Wildcard)
                } else {
                    Ok(Pattern::Identifier(name))
                }
            }
            _ => Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "pattern".to_string(),
                    found: format!("{}", self.current_token),
                },
            }),
        }
    }

    // Parse a block of statements with error recovery
    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.consume(Token::LeftBrace, "Expected '{' to start block")?;
        
        let mut statements = Vec::new();
        let mut errors = Vec::new();
        
        while !matches!(self.current_token, Token::RightBrace) && !self.is_at_end() {
            match self.parse_with_recovery(|p| p.parse_statement_impl(), "statement") {
                Ok(stmt) => statements.push(stmt),
                Err(error) => {
                    errors.push(error);
                    // Continue parsing after error recovery
                }
            }
        }
        
        self.consume(Token::RightBrace, "Expected '}' to end block")?;
        
        // For now, return the first error if any occurred
        if let Some(first_error) = errors.into_iter().next() {
            return Err(first_error);
        }
        
        Ok(Block { statements })
    }

    // Parse a type annotation
    fn parse_type(&mut self) -> Result<Type, ParseError> {
        match &self.current_token {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance()?;
                
                match name.as_str() {
                    "int" => Ok(Type::Int),
                    "float" => Ok(Type::Float),
                    "string" => Ok(Type::String),
                    "bool" => Ok(Type::Bool),
                    "char" => Ok(Type::Char),
                    "byte" => Ok(Type::Byte),
                    _ => Ok(Type::Named(name)),
                }
            }
            Token::LeftBracket => {
                self.advance()?; // consume '['
                let element_type = self.parse_type()?;
                self.consume(Token::RightBracket, "Expected ']' after array element type")?;
                Ok(Type::Array(Box::new(element_type)))
            }
            _ => Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "type".to_string(),
                    found: format!("{}", self.current_token),
                },
            }),
        }
    }
    
    fn parse_function_impl(&mut self) -> Result<Function, ParseError> {
        // Check for visibility modifier
        let visibility = if matches!(self.current_token, Token::Pub) {
            self.advance()?; // consume 'pub'
            Visibility::Public
        } else {
            Visibility::Private
        };

        // Check for async modifier
        let is_async = if matches!(self.current_token, Token::Async) {
            self.advance()?; // consume 'async'
            true
        } else {
            false
        };

        self.consume(Token::Func, "Expected 'func'")?;
        
        let name = if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance()?;
            name
        } else {
            return Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "function name".to_string(),
                    found: format!("{}", self.current_token),
                },
            });
        };
        
        self.consume(Token::LeftParen, "Expected '(' after function name")?;
        
        // Parse parameter list
        let mut parameters = Vec::new();
        if !matches!(self.current_token, Token::RightParen) {
            loop {
                parameters.push(self.parse_parameter()?);
                if matches!(self.current_token, Token::Comma) {
                    self.advance()?; // consume ','
                } else {
                    break;
                }
            }
        }
        
        self.consume(Token::RightParen, "Expected ')' after parameters")?;
        
        // Optional return type
        let return_type = if matches!(self.current_token, Token::Arrow) {
            self.advance()?; // consume '->'
            Some(self.parse_type()?)
        } else {
            None
        };
        
        let body = self.parse_block()?;
        
        Ok(Function {
            name,
            parameters,
            return_type,
            body,
            is_async,
            visibility,
        })
    }

    fn parse_parameter(&mut self) -> Result<Parameter, ParseError> {
        // Check for mutability modifier
        let is_mutable = if matches!(self.current_token, Token::Mut) {
            self.advance()?; // consume 'mut'
            true
        } else {
            false
        };

        if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance()?;
            
            self.consume(Token::Colon, "Expected ':' after parameter name")?;
            let type_ = self.parse_type()?;
            
            Ok(Parameter {
                name,
                type_,
                is_mutable,
            })
        } else {
            Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "parameter name".to_string(),
                    found: format!("{}", self.current_token),
                },
            })
        }
    }
    
    fn parse_struct_impl(&mut self) -> Result<Struct, ParseError> {
        // Check for visibility modifier
        let visibility = if matches!(self.current_token, Token::Pub) {
            self.advance()?; // consume 'pub'
            Visibility::Public
        } else {
            Visibility::Private
        };

        self.consume(Token::Struct, "Expected 'struct'")?;
        
        let name = if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance()?;
            name
        } else {
            return Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "struct name".to_string(),
                    found: format!("{}", self.current_token),
                },
            });
        };
        
        self.consume(Token::LeftBrace, "Expected '{' after struct name")?;
        
        // Parse struct fields
        let mut fields = Vec::new();
        while !matches!(self.current_token, Token::RightBrace) && !self.is_at_end() {
            fields.push(self.parse_field()?);
            
            // Optional comma after field
            if matches!(self.current_token, Token::Comma) {
                self.advance()?;
            }
        }
        
        self.consume(Token::RightBrace, "Expected '}' to end struct")?;
        
        Ok(Struct {
            name,
            fields,
            visibility,
        })
    }

    fn parse_field(&mut self) -> Result<Field, ParseError> {
        // Check for visibility modifier
        let visibility = if matches!(self.current_token, Token::Pub) {
            self.advance()?; // consume 'pub'
            Visibility::Public
        } else {
            Visibility::Private
        };

        // Check for mutability modifier
        let is_mutable = if matches!(self.current_token, Token::Mut) {
            self.advance()?; // consume 'mut'
            true
        } else {
            false
        };

        if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance()?;
            
            self.consume(Token::Colon, "Expected ':' after field name")?;
            let type_ = self.parse_type()?;
            
            Ok(Field {
                name,
                type_,
                visibility,
                is_mutable,
            })
        } else {
            Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "field name".to_string(),
                    found: format!("{}", self.current_token),
                },
            })
        }
    }
    
    fn parse_extern_function_impl(&mut self) -> Result<ExternFunction, ParseError> {
        // Check for visibility modifier
        let visibility = if matches!(self.current_token, Token::Pub) {
            self.advance()?; // consume 'pub'
            Visibility::Public
        } else {
            Visibility::Private
        };

        self.consume(Token::Extern, "Expected 'extern'")?;
        
        // Parse optional library specification: extern "C" or extern "library_name"
        let library = if let Token::String(lib_name) = &self.current_token {
            let lib = Some(lib_name.clone());
            self.advance()?;
            lib
        } else {
            None
        };
        
        self.consume(Token::Func, "Expected 'func' after extern")?;
        
        let name = if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance()?;
            name
        } else {
            return Err(ParseError {
                span: Span::single(self.lexer.position()),
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "function name".to_string(),
                    found: format!("{}", self.current_token),
                },
            });
        };
        
        self.consume(Token::LeftParen, "Expected '(' after function name")?;
        
        // Parse parameter list
        let mut parameters = Vec::new();
        let mut is_variadic = false;
        
        if !matches!(self.current_token, Token::RightParen) {
            loop {
                // Check for variadic parameter (...)
                if matches!(self.current_token, Token::Dot) {
                    // Expect three dots for variadic
                    self.advance()?; // consume first '.'
                    self.consume(Token::Dot, "Expected second '.' for variadic parameter")?;
                    self.consume(Token::Dot, "Expected third '.' for variadic parameter")?;
                    is_variadic = true;
                    break;
                }
                
                parameters.push(self.parse_parameter()?);
                if matches!(self.current_token, Token::Comma) {
                    self.advance()?; // consume ','
                } else {
                    break;
                }
            }
        }
        
        self.consume(Token::RightParen, "Expected ')' after parameters")?;
        
        // Optional return type
        let return_type = if matches!(self.current_token, Token::Arrow) {
            self.advance()?; // consume '->'
            Some(self.parse_type()?)
        } else {
            None
        };
        
        // Extern functions end with semicolon, not a body
        self.consume(Token::Semicolon, "Expected ';' after extern function declaration")?;
        
        Ok(ExternFunction {
            name,
            parameters,
            return_type,
            library,
            is_variadic,
            visibility,
        })
    }
    
    fn is_at_end(&self) -> bool {
        matches!(self.current_token, Token::Eof)
    }
}

impl Parser for FluxParser {
    fn parse_program(&mut self) -> Result<Program, ParseError> {
        self.parse_program_impl()
    }
    
    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_expression_impl()
    }
    
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        self.parse_statement_impl()
    }
    
    fn parse_function(&mut self) -> Result<Function, ParseError> {
        self.parse_function_impl()
    }
    
    fn parse_struct(&mut self) -> Result<Struct, ParseError> {
        self.parse_struct_impl()
    }
    
    fn is_at_end(&self) -> bool {
        matches!(self.current_token, Token::Eof)
    }
}

#[cfg(test)]
mod expression_tests {
    use super::*;
    use crate::lexer::FluxLexer;

    fn parse_expression_from_source(source: &str) -> Result<Expression, ParseError> {
        let lexer = FluxLexer::new(source.to_string());
        let mut parser = FluxParser::new(lexer)?;
        parser.parse_expression()
    }

    #[test]
    fn test_literal_expressions() {
        // Integer literal
        let expr = parse_expression_from_source("42").unwrap();
        assert_eq!(expr, Expression::Literal(Literal::Integer(42)));

        // Float literal
        let expr = parse_expression_from_source("3.14").unwrap();
        assert_eq!(expr, Expression::Literal(Literal::Float(3.14)));

        // String literal
        let expr = parse_expression_from_source("\"hello\"").unwrap();
        assert_eq!(expr, Expression::Literal(Literal::String("hello".to_string())));

        // Boolean literal
        let expr = parse_expression_from_source("true").unwrap();
        assert_eq!(expr, Expression::Literal(Literal::Boolean(true)));

        // Character literal
        let expr = parse_expression_from_source("'a'").unwrap();
        assert_eq!(expr, Expression::Literal(Literal::Character('a')));
    }

    #[test]
    fn test_identifier_expressions() {
        let expr = parse_expression_from_source("variable").unwrap();
        assert_eq!(expr, Expression::Identifier("variable".to_string()));
    }

    #[test]
    fn test_binary_expressions() {
        // Addition
        let expr = parse_expression_from_source("1 + 2").unwrap();
        assert_eq!(expr, Expression::Binary(
            Box::new(Expression::Literal(Literal::Integer(1))),
            BinaryOp::Add,
            Box::new(Expression::Literal(Literal::Integer(2)))
        ));

        // Multiplication with higher precedence
        let expr = parse_expression_from_source("1 + 2 * 3").unwrap();
        assert_eq!(expr, Expression::Binary(
            Box::new(Expression::Literal(Literal::Integer(1))),
            BinaryOp::Add,
            Box::new(Expression::Binary(
                Box::new(Expression::Literal(Literal::Integer(2))),
                BinaryOp::Multiply,
                Box::new(Expression::Literal(Literal::Integer(3)))
            ))
        ));

        // Comparison
        let expr = parse_expression_from_source("x == 5").unwrap();
        assert_eq!(expr, Expression::Binary(
            Box::new(Expression::Identifier("x".to_string())),
            BinaryOp::Equal,
            Box::new(Expression::Literal(Literal::Integer(5)))
        ));

        // Logical AND
        let expr = parse_expression_from_source("true && false").unwrap();
        assert_eq!(expr, Expression::Binary(
            Box::new(Expression::Literal(Literal::Boolean(true))),
            BinaryOp::And,
            Box::new(Expression::Literal(Literal::Boolean(false)))
        ));
    }

    #[test]
    fn test_unary_expressions() {
        // Negation
        let expr = parse_expression_from_source("-42").unwrap();
        assert_eq!(expr, Expression::Unary(
            UnaryOp::Minus,
            Box::new(Expression::Literal(Literal::Integer(42)))
        ));

        // Logical NOT
        let expr = parse_expression_from_source("!true").unwrap();
        assert_eq!(expr, Expression::Unary(
            UnaryOp::Not,
            Box::new(Expression::Literal(Literal::Boolean(true)))
        ));

        // Bitwise NOT
        let expr = parse_expression_from_source("~42").unwrap();
        assert_eq!(expr, Expression::Unary(
            UnaryOp::BitwiseNot,
            Box::new(Expression::Literal(Literal::Integer(42)))
        ));
    }

    #[test]
    fn test_function_call_expressions() {
        // Simple function call
        let expr = parse_expression_from_source("print()").unwrap();
        assert_eq!(expr, Expression::Call(
            Box::new(Expression::Identifier("print".to_string())),
            vec![]
        ));

        // Function call with arguments
        let expr = parse_expression_from_source("add(1, 2)").unwrap();
        assert_eq!(expr, Expression::Call(
            Box::new(Expression::Identifier("add".to_string())),
            vec![
                Expression::Literal(Literal::Integer(1)),
                Expression::Literal(Literal::Integer(2))
            ]
        ));

        // Chained function calls
        let expr = parse_expression_from_source("obj.method()").unwrap();
        assert_eq!(expr, Expression::Call(
            Box::new(Expression::Field(
                Box::new(Expression::Identifier("obj".to_string())),
                "method".to_string()
            )),
            vec![]
        ));
    }

    #[test]
    fn test_field_access_expressions() {
        let expr = parse_expression_from_source("obj.field").unwrap();
        assert_eq!(expr, Expression::Field(
            Box::new(Expression::Identifier("obj".to_string())),
            "field".to_string()
        ));

        // Chained field access
        let expr = parse_expression_from_source("obj.field.subfield").unwrap();
        assert_eq!(expr, Expression::Field(
            Box::new(Expression::Field(
                Box::new(Expression::Identifier("obj".to_string())),
                "field".to_string()
            )),
            "subfield".to_string()
        ));
    }

    #[test]
    fn test_index_expressions() {
        // Array indexing
        let expr = parse_expression_from_source("arr[0]").unwrap();
        assert_eq!(expr, Expression::Index(
            Box::new(Expression::Identifier("arr".to_string())),
            Box::new(Expression::Literal(Literal::Integer(0)))
        ));

        // Map indexing
        let expr = parse_expression_from_source("map[\"key\"]").unwrap();
        assert_eq!(expr, Expression::Index(
            Box::new(Expression::Identifier("map".to_string())),
            Box::new(Expression::Literal(Literal::String("key".to_string())))
        ));

        // Chained indexing
        let expr = parse_expression_from_source("matrix[i][j]").unwrap();
        assert_eq!(expr, Expression::Index(
            Box::new(Expression::Index(
                Box::new(Expression::Identifier("matrix".to_string())),
                Box::new(Expression::Identifier("i".to_string()))
            )),
            Box::new(Expression::Identifier("j".to_string()))
        ));
    }

    #[test]
    fn test_array_literal_expressions() {
        // Empty array
        let expr = parse_expression_from_source("[]").unwrap();
        assert_eq!(expr, Expression::Array(vec![]));

        // Array with elements
        let expr = parse_expression_from_source("[1, 2, 3]").unwrap();
        assert_eq!(expr, Expression::Array(vec![
            Expression::Literal(Literal::Integer(1)),
            Expression::Literal(Literal::Integer(2)),
            Expression::Literal(Literal::Integer(3))
        ]));
    }

    #[test]
    fn test_parenthesized_expressions() {
        let expr = parse_expression_from_source("(1 + 2)").unwrap();
        assert_eq!(expr, Expression::Binary(
            Box::new(Expression::Literal(Literal::Integer(1))),
            BinaryOp::Add,
            Box::new(Expression::Literal(Literal::Integer(2)))
        ));

        // Parentheses changing precedence
        let expr = parse_expression_from_source("(1 + 2) * 3").unwrap();
        assert_eq!(expr, Expression::Binary(
            Box::new(Expression::Binary(
                Box::new(Expression::Literal(Literal::Integer(1))),
                BinaryOp::Add,
                Box::new(Expression::Literal(Literal::Integer(2)))
            )),
            BinaryOp::Multiply,
            Box::new(Expression::Literal(Literal::Integer(3)))
        ));
    }

    #[test]
    fn test_complex_expressions() {
        // Complex expression with multiple operators and precedence
        let expr = parse_expression_from_source("a + b * c == d && e || f").unwrap();
        
        // This should parse as: ((a + (b * c)) == d) && e) || f
        // Due to operator precedence: *, +, ==, &&, ||
        match expr {
            Expression::Binary(_, BinaryOp::Or, _) => {
                // Top level should be OR
            }
            _ => panic!("Expected OR at top level"),
        }
    }

    #[test]
    fn test_operator_precedence() {
        // Test that multiplication has higher precedence than addition
        let expr = parse_expression_from_source("2 + 3 * 4").unwrap();
        match expr {
            Expression::Binary(left, BinaryOp::Add, right) => {
                assert_eq!(*left, Expression::Literal(Literal::Integer(2)));
                match *right {
                    Expression::Binary(_, BinaryOp::Multiply, _) => {
                        // Correct: 3 * 4 is grouped together
                    }
                    _ => panic!("Expected multiplication to have higher precedence"),
                }
            }
            _ => panic!("Expected addition at top level"),
        }

        // Test that comparison has lower precedence than arithmetic
        let expr = parse_expression_from_source("1 + 2 == 3").unwrap();
        match expr {
            Expression::Binary(left, BinaryOp::Equal, right) => {
                match *left {
                    Expression::Binary(_, BinaryOp::Add, _) => {
                        // Correct: 1 + 2 is grouped together
                    }
                    _ => panic!("Expected addition to have higher precedence"),
                }
                assert_eq!(*right, Expression::Literal(Literal::Integer(3)));
            }
            _ => panic!("Expected equality at top level"),
        }
    }

    #[test]
    fn test_associativity() {
        // Test left associativity for same precedence operators
        let expr = parse_expression_from_source("1 - 2 - 3").unwrap();
        match expr {
            Expression::Binary(left, BinaryOp::Subtract, right) => {
                match *left {
                    Expression::Binary(_, BinaryOp::Subtract, _) => {
                        // Correct: (1 - 2) - 3
                    }
                    _ => panic!("Expected left associativity"),
                }
                assert_eq!(*right, Expression::Literal(Literal::Integer(3)));
            }
            _ => panic!("Expected subtraction at top level"),
        }
    }

    #[test]
    fn test_error_cases() {
        // Invalid expression
        assert!(parse_expression_from_source("").is_err());
        
        // Unclosed parentheses
        assert!(parse_expression_from_source("(1 + 2").is_err());
        
        // Invalid binary operator usage (missing right operand)
        assert!(parse_expression_from_source("1 +").is_err());
    }
}#[cfg(
test)]
mod statement_tests {
    use super::*;
    use crate::lexer::FluxLexer;

    fn parse_statement_from_source(source: &str) -> Result<Statement, ParseError> {
        let lexer = FluxLexer::new(source.to_string());
        let mut parser = FluxParser::new(lexer)?;
        parser.parse_statement()
    }

    fn parse_function_from_source(source: &str) -> Result<Function, ParseError> {
        let lexer = FluxLexer::new(source.to_string());
        let mut parser = FluxParser::new(lexer)?;
        parser.parse_function()
    }

    fn parse_struct_from_source(source: &str) -> Result<Struct, ParseError> {
        let lexer = FluxLexer::new(source.to_string());
        let mut parser = FluxParser::new(lexer)?;
        parser.parse_struct()
    }

    #[test]
    fn test_let_statements() {
        // Simple let with initialization
        let stmt = parse_statement_from_source("let x = 42").unwrap();
        assert_eq!(stmt, Statement::Let(
            "x".to_string(),
            None,
            Some(Expression::Literal(Literal::Integer(42)))
        ));

        // Let with type annotation
        let stmt = parse_statement_from_source("let x: int = 42").unwrap();
        assert_eq!(stmt, Statement::Let(
            "x".to_string(),
            Some(Type::Int),
            Some(Expression::Literal(Literal::Integer(42)))
        ));

        // Let without initialization
        let stmt = parse_statement_from_source("let x: int").unwrap();
        assert_eq!(stmt, Statement::Let(
            "x".to_string(),
            Some(Type::Int),
            None
        ));
    }

    #[test]
    fn test_const_statements() {
        let stmt = parse_statement_from_source("const PI: float = 3.14").unwrap();
        assert_eq!(stmt, Statement::Const(
            "PI".to_string(),
            Type::Float,
            Expression::Literal(Literal::Float(3.14))
        ));
    }

    #[test]
    fn test_return_statements() {
        // Return with value
        let stmt = parse_statement_from_source("return 42").unwrap();
        assert_eq!(stmt, Statement::Return(Some(Expression::Literal(Literal::Integer(42)))));

        // Return without value
        let stmt = parse_statement_from_source("return").unwrap();
        assert_eq!(stmt, Statement::Return(None));
    }

    #[test]
    fn test_assignment_statements() {
        let stmt = parse_statement_from_source("x = 42").unwrap();
        assert_eq!(stmt, Statement::Assignment(
            Expression::Identifier("x".to_string()),
            Expression::Literal(Literal::Integer(42))
        ));
    }

    #[test]
    fn test_control_flow_statements() {
        // Break statement
        let stmt = parse_statement_from_source("break").unwrap();
        assert_eq!(stmt, Statement::Break(None));

        // Continue statement
        let stmt = parse_statement_from_source("continue").unwrap();
        assert_eq!(stmt, Statement::Continue);

        // Go statement
        let stmt = parse_statement_from_source("go print()").unwrap();
        assert_eq!(stmt, Statement::Go(Expression::Call(
            Box::new(Expression::Identifier("print".to_string())),
            vec![]
        )));
    }

    #[test]
    fn test_if_statements() {
        let stmt = parse_statement_from_source("if x > 0 { return x }").unwrap();
        match stmt {
            Statement::If(condition, then_block, else_block) => {
                assert!(matches!(condition, Expression::Binary(_, BinaryOp::Greater, _)));
                assert_eq!(then_block.statements.len(), 1);
                assert!(else_block.is_none());
            }
            _ => panic!("Expected if statement"),
        }

        // If-else statement
        let stmt = parse_statement_from_source("if x > 0 { return x } else { return 0 }").unwrap();
        match stmt {
            Statement::If(_, _, else_block) => {
                assert!(else_block.is_some());
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn test_while_statements() {
        let stmt = parse_statement_from_source("while x > 0 { x = x - 1 }").unwrap();
        match stmt {
            Statement::While(condition, body) => {
                assert!(matches!(condition, Expression::Binary(_, BinaryOp::Greater, _)));
                assert_eq!(body.statements.len(), 1);
            }
            _ => panic!("Expected while statement"),
        }
    }

    #[test]
    fn test_for_statements() {
        let stmt = parse_statement_from_source("for i in range { print(i) }").unwrap();
        match stmt {
            Statement::For(var, iterable, body) => {
                assert_eq!(var, "i");
                assert_eq!(iterable, Expression::Identifier("range".to_string()));
                assert_eq!(body.statements.len(), 1);
            }
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn test_expression_statements() {
        let stmt = parse_statement_from_source("print(42)").unwrap();
        assert_eq!(stmt, Statement::Expression(Expression::Call(
            Box::new(Expression::Identifier("print".to_string())),
            vec![Expression::Literal(Literal::Integer(42))]
        )));
    }

    #[test]
    fn test_function_declarations() {
        // Simple function
        let func = parse_function_from_source("func hello() { print(\"Hello\") }").unwrap();
        assert_eq!(func.name, "hello");
        assert!(func.parameters.is_empty());
        assert!(func.return_type.is_none());
        assert_eq!(func.visibility, Visibility::Private);
        assert!(!func.is_async);

        // Function with parameters and return type
        let func = parse_function_from_source("pub func add(a: int, b: int) -> int { return a + b }").unwrap();
        assert_eq!(func.name, "add");
        assert_eq!(func.parameters.len(), 2);
        assert_eq!(func.parameters[0].name, "a");
        assert_eq!(func.parameters[0].type_, Type::Int);
        assert_eq!(func.parameters[1].name, "b");
        assert_eq!(func.parameters[1].type_, Type::Int);
        assert_eq!(func.return_type, Some(Type::Int));
        assert_eq!(func.visibility, Visibility::Public);

        // Async function
        let func = parse_function_from_source("async func fetch() { }").unwrap();
        assert!(func.is_async);
    }

    #[test]
    fn test_struct_declarations() {
        // Empty struct
        let struct_ = parse_struct_from_source("struct Point { }").unwrap();
        assert_eq!(struct_.name, "Point");
        assert!(struct_.fields.is_empty());
        assert_eq!(struct_.visibility, Visibility::Private);

        // Struct with fields
        let struct_ = parse_struct_from_source("pub struct Point { pub x: int, pub y: int }").unwrap();
        assert_eq!(struct_.name, "Point");
        assert_eq!(struct_.fields.len(), 2);
        assert_eq!(struct_.fields[0].name, "x");
        assert_eq!(struct_.fields[0].type_, Type::Int);
        assert_eq!(struct_.fields[0].visibility, Visibility::Public);
        assert_eq!(struct_.fields[1].name, "y");
        assert_eq!(struct_.fields[1].type_, Type::Int);
        assert_eq!(struct_.visibility, Visibility::Public);

        // Struct with mutable field
        let struct_ = parse_struct_from_source("struct Counter { mut count: int }").unwrap();
        assert_eq!(struct_.fields[0].name, "count");
        assert!(struct_.fields[0].is_mutable);
    }

    #[test]
    fn test_type_parsing() {
        // Test various type parsing through let statements
        let stmt = parse_statement_from_source("let arr: [int]").unwrap();
        match stmt {
            Statement::Let(_, Some(Type::Array(element_type)), _) => {
                assert_eq!(*element_type, Type::Int);
            }
            _ => panic!("Expected let statement with array type"),
        }

        let stmt = parse_statement_from_source("let name: string").unwrap();
        match stmt {
            Statement::Let(_, Some(Type::String), _) => {}
            _ => panic!("Expected let statement with string type"),
        }

        let stmt = parse_statement_from_source("let custom: MyType").unwrap();
        match stmt {
            Statement::Let(_, Some(Type::Named(name)), _) => {
                assert_eq!(name, "MyType");
            }
            _ => panic!("Expected let statement with named type"),
        }
    }

    #[test]
    fn test_complex_statements() {
        // Nested if statements
        let stmt = parse_statement_from_source("if x > 0 { if y > 0 { return x + y } }").unwrap();
        match stmt {
            Statement::If(_, then_block, _) => {
                assert_eq!(then_block.statements.len(), 1);
                match &then_block.statements[0] {
                    Statement::If(_, _, _) => {} // Nested if
                    _ => panic!("Expected nested if statement"),
                }
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn test_error_cases() {
        // Invalid let statement
        assert!(parse_statement_from_source("let").is_err());
        
        // Invalid const statement
        assert!(parse_statement_from_source("const x").is_err());
        
        // Invalid function declaration
        assert!(parse_function_from_source("func").is_err());
        
        // Invalid struct declaration
        assert!(parse_struct_from_source("struct").is_err());
    }
}

#[cfg(test)]
mod error_recovery_tests {
    use super::*;
    use crate::lexer::FluxLexer;

    fn parse_program_from_source(source: &str) -> Result<Program, ParseError> {
        let lexer = FluxLexer::new(source.to_string());
        let mut parser = FluxParser::new(lexer)?;
        parser.parse_program()
    }

    #[test]
    fn test_error_messages_with_context() {
        // Test that error messages include helpful context
        let result = parse_program_from_source("func invalid_func(");
        assert!(result.is_err());
        let error = result.unwrap_err();
        match error.kind {
            ParseErrorKind::UnexpectedToken { expected, found: _ } => {
                assert!(expected.contains("while parsing"));
            }
            _ => panic!("Expected UnexpectedToken error"),
        }
    }

    #[test]
    fn test_synchronization_points() {
        // Test that parser can recover from errors at statement boundaries
        let source = r#"
            func valid_func() {
                let x = 42
            }
            
            func invalid_func(
            
            func another_valid_func() {
                let y = 24
            }
        "#;
        
        // Even though there's an error in invalid_func, we should still be able to
        // parse some valid functions (though this specific test might fail due to
        // our current error handling - this is more of a design goal)
        let result = parse_program_from_source(source);
        // For now, we expect this to fail, but in a more robust implementation
        // it would recover and parse the valid functions
        assert!(result.is_err());
    }

    #[test]
    fn test_helpful_error_suggestions() {
        // Test missing semicolon
        let result = parse_program_from_source("func test() { let x = 42 let y = 24 }");
        assert!(result.is_err());

        // Test unclosed parentheses
        let result = parse_program_from_source("func test(param: int { }");
        assert!(result.is_err());

        // Test missing return type arrow
        let result = parse_program_from_source("func test() int { }");
        assert!(result.is_err());
    }

    #[test]
    fn test_expression_error_recovery() {
        // Test that expression parsing can handle various error cases
        let lexer = FluxLexer::new("1 + + 2".to_string());
        let mut parser = FluxParser::new(lexer).unwrap();
        let result = parser.parse_expression();
        assert!(result.is_err());

        // Test unclosed parentheses in expressions
        let lexer = FluxLexer::new("(1 + 2".to_string());
        let mut parser = FluxParser::new(lexer).unwrap();
        let result = parser.parse_expression();
        assert!(result.is_err());

        // Test invalid function call
        let lexer = FluxLexer::new("func(".to_string());
        let mut parser = FluxParser::new(lexer).unwrap();
        let result = parser.parse_expression();
        assert!(result.is_err());
    }

    #[test]
    fn test_statement_error_recovery() {
        // Test various statement parsing errors
        let lexer = FluxLexer::new("let".to_string());
        let mut parser = FluxParser::new(lexer).unwrap();
        let result = parser.parse_statement();
        assert!(result.is_err());

        let lexer = FluxLexer::new("const x".to_string());
        let mut parser = FluxParser::new(lexer).unwrap();
        let result = parser.parse_statement();
        assert!(result.is_err());

        let lexer = FluxLexer::new("if".to_string());
        let mut parser = FluxParser::new(lexer).unwrap();
        let result = parser.parse_statement();
        assert!(result.is_err());
    }

    #[test]
    fn test_function_declaration_errors() {
        // Missing function name
        let result = parse_program_from_source("func () { }");
        assert!(result.is_err());

        // Missing parameter type
        let result = parse_program_from_source("func test(param) { }");
        assert!(result.is_err());

        // Invalid return type
        let result = parse_program_from_source("func test() -> { }");
        assert!(result.is_err());

        // Missing function body
        let result = parse_program_from_source("func test()");
        assert!(result.is_err());
    }

    #[test]
    fn test_struct_declaration_errors() {
        // Missing struct name
        let result = parse_program_from_source("struct { }");
        assert!(result.is_err());

        // Invalid field declaration
        let result = parse_program_from_source("struct Test { field }");
        assert!(result.is_err());

        // Missing field type
        let result = parse_program_from_source("struct Test { field: }");
        assert!(result.is_err());
    }

    #[test]
    fn test_type_parsing_errors() {
        // Invalid array type
        let result = parse_program_from_source("let x: [");
        assert!(result.is_err());

        // Missing array element type
        let result = parse_program_from_source("let x: []");
        assert!(result.is_err());
    }

    #[test]
    fn test_block_parsing_errors() {
        // Unclosed block
        let result = parse_program_from_source("func test() { let x = 42");
        assert!(result.is_err());

        // Invalid statement in block
        let result = parse_program_from_source("func test() { invalid_keyword }");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_errors_in_program() {
        // Test a program with multiple syntax errors
        let source = r#"
            func invalid1(
            struct invalid2 {
            func valid() { let x = 42 }
            const invalid3
        "#;
        
        let result = parse_program_from_source(source);
        assert!(result.is_err());
        // In a more sophisticated implementation, we would collect all errors
        // and still parse the valid function
    }

    #[test]
    fn test_error_position_tracking() {
        // Test that errors include position information
        let result = parse_program_from_source("func test( invalid");
        assert!(result.is_err());
        let error = result.unwrap_err();
        // The error should have position information
        assert!(error.span.start.line > 0 || error.span.start.column > 0);
    }
}