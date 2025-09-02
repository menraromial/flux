//! Abstract Syntax Tree definitions for the Flux language
//! 
//! Defines all AST node types that represent the structure of Flux programs.

use std::fmt;

/// Root node representing a complete Flux program
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub package: String,
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

/// Import declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
}

/// Top-level item in a program
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(Function),
    Struct(Struct),
    Class(Class),
    Const(Const),
    ExternFunction(ExternFunction),
}

/// External function declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ExternFunction {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub library: Option<String>,
    pub is_variadic: bool,
    pub visibility: Visibility,
}

/// Function declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub is_async: bool,
    pub visibility: Visibility,
}

/// Function parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_: Type,
    pub is_mutable: bool,
}

/// Struct declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Field>,
    pub visibility: Visibility,
}

/// Class declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub name: String,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub visibility: Visibility,
}

/// Struct or class field
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub type_: Type,
    pub visibility: Visibility,
    pub is_mutable: bool,
}

/// Class method
#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub visibility: Visibility,
    pub is_static: bool,
}

/// Constant declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Const {
    pub name: String,
    pub type_: Type,
    pub value: Expression,
    pub visibility: Visibility,
}

/// Visibility modifier
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

/// Type annotation
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Primitive types
    Int,
    Float,
    String,
    Bool,
    Char,
    Byte,
    
    // Compound types
    Array(Box<Type>),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Set(Box<Type>),
    
    // User-defined types
    Named(String),
    
    // Function types
    Function(Vec<Type>, Box<Type>),
    
    // Generic types
    Generic(String, Vec<Type>),
    
    // Special types
    Nullable(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Unit,
    Never,
}

/// Block of statements
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
}

/// Statement
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Expression),
    Let(String, Option<Type>, Option<Expression>),
    Const(String, Type, Expression),
    Assignment(Expression, Expression),
    Return(Option<Expression>),
    Break(Option<Expression>),
    Continue,
    Go(Expression),
    If(Expression, Block, Option<Block>),
    While(Expression, Block),
    For(String, Expression, Block),
    Match(Expression, Vec<MatchArm>),
}

/// Match arm
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expression>,
    pub body: Block,
}

/// Pattern for match expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(Literal),
    Identifier(String),
    Wildcard,
    Tuple(Vec<Pattern>),
    Struct(String, Vec<(String, Pattern)>),
    Result(ResultPattern),
}

/// Pattern for matching Result types
#[derive(Debug, Clone, PartialEq)]
pub enum ResultPattern {
    Ok(Box<Pattern>),
    Err(Box<Pattern>),
}

/// Expression
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    Binary(Box<Expression>, BinaryOp, Box<Expression>),
    Unary(UnaryOp, Box<Expression>),
    Call(Box<Expression>, Vec<Expression>),
    Index(Box<Expression>, Box<Expression>),
    Field(Box<Expression>, String),
    Match(Box<Expression>, Vec<MatchArm>),
    If(Box<Expression>, Block, Option<Block>),
    Block(Block),
    Array(Vec<Expression>),
    Map(Vec<(Expression, Expression)>),
    Tuple(Vec<Expression>),
}

/// Literal value
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Character(char),
    Null,
}

/// Binary operator
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    
    // Comparison
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    
    // Logical
    And,
    Or,
    
    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
}

/// Unary operator
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    BitwiseNot,
    Try, // The ? operator for error propagation
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "package {}", self.package)?;
        
        for import in &self.imports {
            writeln!(f, "{}", import)?;
        }
        
        if !self.imports.is_empty() {
            writeln!(f)?;
        }
        
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 { writeln!(f)?; }
            write!(f, "{}", item)?;
        }
        
        Ok(())
    }
}

impl fmt::Display for Import {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "import \"{}\"", self.path)?;
        if let Some(alias) = &self.alias {
            write!(f, " as {}", alias)?;
        }
        Ok(())
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Function(func) => write!(f, "{}", func),
            Item::Struct(struct_) => write!(f, "{}", struct_),
            Item::Class(class) => write!(f, "{}", class),
            Item::Const(const_) => write!(f, "{}", const_),
            Item::ExternFunction(extern_func) => write!(f, "{}", extern_func),
        }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.visibility)?;
        if self.is_async {
            write!(f, "async ")?;
        }
        write!(f, "func {}(", self.name)?;
        
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", param)?;
        }
        
        write!(f, ")")?;
        
        if let Some(ret_type) = &self.return_type {
            write!(f, " -> {}", ret_type)?;
        }
        
        write!(f, " {}", self.body)
    }
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_mutable {
            write!(f, "mut ")?;
        }
        write!(f, "{}: {}", self.name, self.type_)
    }
}

impl fmt::Display for Struct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}struct {} {{\n", self.visibility, self.name)?;
        
        for field in &self.fields {
            writeln!(f, "    {}", field)?;
        }
        
        write!(f, "}}")
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}class {} {{\n", self.visibility, self.name)?;
        
        for field in &self.fields {
            writeln!(f, "    {}", field)?;
        }
        
        if !self.fields.is_empty() && !self.methods.is_empty() {
            writeln!(f)?;
        }
        
        for method in &self.methods {
            writeln!(f, "    {}", method)?;
        }
        
        write!(f, "}}")
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.visibility)?;
        if self.is_mutable {
            write!(f, "mut ")?;
        }
        write!(f, "{}: {}", self.name, self.type_)
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.visibility)?;
        if self.is_static {
            write!(f, "static ")?;
        }
        write!(f, "func {}(", self.name)?;
        
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", param)?;
        }
        
        write!(f, ")")?;
        
        if let Some(ret_type) = &self.return_type {
            write!(f, " -> {}", ret_type)?;
        }
        
        write!(f, " {}", self.body)
    }
}

impl fmt::Display for Const {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}const {}: {} = {}", 
               self.visibility, self.name, self.type_, self.value)
    }
}

impl fmt::Display for ExternFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}extern", self.visibility)?;
        
        if let Some(lib) = &self.library {
            write!(f, " \"{}\"", lib)?;
        }
        
        write!(f, " func {}(", self.name)?;
        
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }
        
        if self.is_variadic {
            if !self.parameters.is_empty() {
                write!(f, ", ")?;
            }
            write!(f, "...")?;
        }
        
        write!(f, ")")?;
        
        if let Some(ret_type) = &self.return_type {
            write!(f, " -> {}", ret_type)?;
        }
        
        write!(f, ";")
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public => write!(f, "pub "),
            Visibility::Private => Ok(()),
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{{")?;
        for stmt in &self.statements {
            writeln!(f, "    {}", stmt)?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::Expression(expr) => write!(f, "{}", expr),
            Statement::Let(name, type_, value) => {
                write!(f, "let {}", name)?;
                if let Some(t) = type_ {
                    write!(f, ": {}", t)?;
                }
                if let Some(v) = value {
                    write!(f, " = {}", v)?;
                }
                Ok(())
            }
            Statement::Const(name, type_, value) => {
                write!(f, "const {}: {} = {}", name, type_, value)
            }
            Statement::Assignment(target, value) => {
                write!(f, "{} = {}", target, value)
            }
            Statement::Return(value) => {
                write!(f, "return")?;
                if let Some(v) = value {
                    write!(f, " {}", v)?;
                }
                Ok(())
            }
            Statement::Break(value) => {
                write!(f, "break")?;
                if let Some(v) = value {
                    write!(f, " {}", v)?;
                }
                Ok(())
            }
            Statement::Continue => write!(f, "continue"),
            Statement::Go(expr) => write!(f, "go {}", expr),
            Statement::If(cond, then_block, else_block) => {
                write!(f, "if {} {}", cond, then_block)?;
                if let Some(else_b) = else_block {
                    write!(f, " else {}", else_b)?;
                }
                Ok(())
            }
            Statement::While(cond, body) => {
                write!(f, "while {} {}", cond, body)
            }
            Statement::For(var, iter, body) => {
                write!(f, "for {} in {} {}", var, iter, body)
            }
            Statement::Match(expr, arms) => {
                writeln!(f, "match {} {{", expr)?;
                for arm in arms {
                    writeln!(f, "    {}", arm)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl fmt::Display for MatchArm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pattern)?;
        if let Some(guard) = &self.guard {
            write!(f, " if {}", guard)?;
        }
        write!(f, " => {}", self.body)
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Literal(lit) => write!(f, "{}", lit),
            Pattern::Identifier(name) => write!(f, "{}", name),
            Pattern::Wildcard => write!(f, "_"),
            Pattern::Tuple(patterns) => {
                write!(f, "(")?;
                for (i, pattern) in patterns.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", pattern)?;
                }
                write!(f, ")")
            }
            Pattern::Struct(name, fields) => {
                write!(f, "{} {{ ", name)?;
                for (i, (field_name, pattern)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", field_name, pattern)?;
                }
                write!(f, " }}")
            }
            Pattern::Result(result_pattern) => write!(f, "{}", result_pattern),
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Literal(lit) => write!(f, "{}", lit),
            Expression::Identifier(name) => write!(f, "{}", name),
            Expression::Binary(left, op, right) => {
                write!(f, "({} {} {})", left, op, right)
            }
            Expression::Unary(op, expr) => {
                write!(f, "{}{}", op, expr)
            }
            Expression::Call(func, args) => {
                write!(f, "{}(", func)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Expression::Index(expr, index) => {
                write!(f, "{}[{}]", expr, index)
            }
            Expression::Field(expr, field) => {
                write!(f, "{}.{}", expr, field)
            }
            Expression::Match(expr, arms) => {
                writeln!(f, "match {} {{", expr)?;
                for arm in arms {
                    writeln!(f, "    {}", arm)?;
                }
                write!(f, "}}")
            }
            Expression::If(cond, then_block, else_block) => {
                write!(f, "if {} {}", cond, then_block)?;
                if let Some(else_b) = else_block {
                    write!(f, " else {}", else_b)?;
                }
                Ok(())
            }
            Expression::Block(block) => write!(f, "{}", block),
            Expression::Array(elements) => {
                write!(f, "[")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Expression::Map(pairs) => {
                write!(f, "{{")?;
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
            Expression::Tuple(elements) => {
                write!(f, "(")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Integer(n) => write!(f, "{}", n),
            Literal::Float(n) => write!(f, "{}", n),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Boolean(b) => write!(f, "{}", b),
            Literal::Character(c) => write!(f, "'{}'", c),
            Literal::Null => write!(f, "null"),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Subtract => write!(f, "-"),
            BinaryOp::Multiply => write!(f, "*"),
            BinaryOp::Divide => write!(f, "/"),
            BinaryOp::Modulo => write!(f, "%"),
            BinaryOp::Equal => write!(f, "=="),
            BinaryOp::NotEqual => write!(f, "!="),
            BinaryOp::Less => write!(f, "<"),
            BinaryOp::Greater => write!(f, ">"),
            BinaryOp::LessEqual => write!(f, "<="),
            BinaryOp::GreaterEqual => write!(f, ">="),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
            BinaryOp::BitwiseAnd => write!(f, "&"),
            BinaryOp::BitwiseOr => write!(f, "|"),
            BinaryOp::BitwiseXor => write!(f, "^"),
            BinaryOp::LeftShift => write!(f, "<<"),
            BinaryOp::RightShift => write!(f, ">>"),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Plus => write!(f, "+"),
            UnaryOp::Minus => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::BitwiseNot => write!(f, "~"),
            UnaryOp::Try => write!(f, "?"),
        }
    }
}

impl fmt::Display for ResultPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResultPattern::Ok(pattern) => write!(f, "Ok({})", pattern),
            ResultPattern::Err(pattern) => write!(f, "Err({})", pattern),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::Byte => write!(f, "byte"),
            Type::Array(t) => write!(f, "[{}]", t),
            Type::List(t) => write!(f, "List<{}>", t),
            Type::Map(k, v) => write!(f, "Map<{}, {}>", k, v),
            Type::Set(t) => write!(f, "Set<{}>", t),
            Type::Named(name) => write!(f, "{}", name),
            Type::Function(params, ret) => {
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Generic(name, args) => {
                write!(f, "{}", name)?;
                if !args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Nullable(t) => write!(f, "{}?", t),
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::Unit => write!(f, "()"),
            Type::Never => write!(f, "!"),
        }
    }
}
#[cfg(test)
]
mod tests {
    use super::*;

    #[test]
    fn test_literal_display() {
        assert_eq!(format!("{}", Literal::Integer(42)), "42");
        assert_eq!(format!("{}", Literal::Float(3.14)), "3.14");
        assert_eq!(format!("{}", Literal::String("hello".to_string())), "\"hello\"");
        assert_eq!(format!("{}", Literal::Boolean(true)), "true");
        assert_eq!(format!("{}", Literal::Character('a')), "'a'");
        assert_eq!(format!("{}", Literal::Null), "null");
    }

    #[test]
    fn test_binary_op_display() {
        assert_eq!(format!("{}", BinaryOp::Add), "+");
        assert_eq!(format!("{}", BinaryOp::Equal), "==");
        assert_eq!(format!("{}", BinaryOp::And), "&&");
        assert_eq!(format!("{}", BinaryOp::BitwiseOr), "|");
    }

    #[test]
    fn test_unary_op_display() {
        assert_eq!(format!("{}", UnaryOp::Plus), "+");
        assert_eq!(format!("{}", UnaryOp::Minus), "-");
        assert_eq!(format!("{}", UnaryOp::Not), "!");
        assert_eq!(format!("{}", UnaryOp::BitwiseNot), "~");
    }

    #[test]
    fn test_type_display() {
        assert_eq!(format!("{}", Type::Int), "int");
        assert_eq!(format!("{}", Type::String), "string");
        assert_eq!(format!("{}", Type::Array(Box::new(Type::Int))), "[int]");
        assert_eq!(format!("{}", Type::List(Box::new(Type::String))), "List<string>");
        assert_eq!(format!("{}", Type::Map(Box::new(Type::String), Box::new(Type::Int))), "Map<string, int>");
        assert_eq!(format!("{}", Type::Nullable(Box::new(Type::Int))), "int?");
        assert_eq!(format!("{}", Type::Result(Box::new(Type::Int), Box::new(Type::String))), "Result<int, string>");
    }

    #[test]
    fn test_expression_display() {
        let expr = Expression::Binary(
            Box::new(Expression::Literal(Literal::Integer(1))),
            BinaryOp::Add,
            Box::new(Expression::Literal(Literal::Integer(2)))
        );
        assert_eq!(format!("{}", expr), "(1 + 2)");

        let call_expr = Expression::Call(
            Box::new(Expression::Identifier("print".to_string())),
            vec![Expression::Literal(Literal::String("hello".to_string()))]
        );
        assert_eq!(format!("{}", call_expr), "print(\"hello\")");

        let field_expr = Expression::Field(
            Box::new(Expression::Identifier("obj".to_string())),
            "field".to_string()
        );
        assert_eq!(format!("{}", field_expr), "obj.field");
    }

    #[test]
    fn test_statement_display() {
        let let_stmt = Statement::Let(
            "x".to_string(),
            Some(Type::Int),
            Some(Expression::Literal(Literal::Integer(42)))
        );
        assert_eq!(format!("{}", let_stmt), "let x: int = 42");

        let return_stmt = Statement::Return(Some(Expression::Identifier("x".to_string())));
        assert_eq!(format!("{}", return_stmt), "return x");

        let assignment = Statement::Assignment(
            Expression::Identifier("x".to_string()),
            Expression::Literal(Literal::Integer(10))
        );
        assert_eq!(format!("{}", assignment), "x = 10");
    }

    #[test]
    fn test_function_display() {
        let func = Function {
            name: "add".to_string(),
            parameters: vec![
                Parameter {
                    name: "a".to_string(),
                    type_: Type::Int,
                    is_mutable: false,
                },
                Parameter {
                    name: "b".to_string(),
                    type_: Type::Int,
                    is_mutable: false,
                }
            ],
            return_type: Some(Type::Int),
            body: Block {
                statements: vec![
                    Statement::Return(Some(Expression::Binary(
                        Box::new(Expression::Identifier("a".to_string())),
                        BinaryOp::Add,
                        Box::new(Expression::Identifier("b".to_string()))
                    )))
                ]
            },
            is_async: false,
            visibility: Visibility::Public,
        };

        let display = format!("{}", func);
        assert!(display.contains("pub func add(a: int, b: int) -> int"));
        assert!(display.contains("return (a + b)"));
    }

    #[test]
    fn test_struct_display() {
        let struct_ = Struct {
            name: "Point".to_string(),
            fields: vec![
                Field {
                    name: "x".to_string(),
                    type_: Type::Int,
                    visibility: Visibility::Public,
                    is_mutable: false,
                },
                Field {
                    name: "y".to_string(),
                    type_: Type::Int,
                    visibility: Visibility::Public,
                    is_mutable: false,
                }
            ],
            visibility: Visibility::Public,
        };

        let display = format!("{}", struct_);
        assert!(display.contains("pub struct Point"));
        assert!(display.contains("pub x: int"));
        assert!(display.contains("pub y: int"));
    }

    #[test]
    fn test_program_display() {
        let program = Program {
            package: "main".to_string(),
            imports: vec![
                Import {
                    path: "std/io".to_string(),
                    alias: None,
                }
            ],
            items: vec![
                Item::Function(Function {
                    name: "main".to_string(),
                    parameters: vec![],
                    return_type: None,
                    body: Block {
                        statements: vec![
                            Statement::Expression(Expression::Call(
                                Box::new(Expression::Identifier("println".to_string())),
                                vec![Expression::Literal(Literal::String("Hello, World!".to_string()))]
                            ))
                        ]
                    },
                    is_async: false,
                    visibility: Visibility::Private,
                })
            ],
        };

        let display = format!("{}", program);
        assert!(display.contains("package main"));
        assert!(display.contains("import \"std/io\""));
        assert!(display.contains("func main()"));
        assert!(display.contains("println(\"Hello, World!\")"));
    }

    #[test]
    fn test_pattern_display() {
        let literal_pattern = Pattern::Literal(Literal::Integer(42));
        assert_eq!(format!("{}", literal_pattern), "42");

        let wildcard_pattern = Pattern::Wildcard;
        assert_eq!(format!("{}", wildcard_pattern), "_");

        let tuple_pattern = Pattern::Tuple(vec![
            Pattern::Identifier("x".to_string()),
            Pattern::Wildcard
        ]);
        assert_eq!(format!("{}", tuple_pattern), "(x, _)");

        let struct_pattern = Pattern::Struct(
            "Point".to_string(),
            vec![
                ("x".to_string(), Pattern::Identifier("px".to_string())),
                ("y".to_string(), Pattern::Wildcard)
            ]
        );
        assert_eq!(format!("{}", struct_pattern), "Point { x: px, y: _ }");
    }

    #[test]
    fn test_match_arm_display() {
        let arm = MatchArm {
            pattern: Pattern::Literal(Literal::Integer(1)),
            guard: Some(Expression::Binary(
                Box::new(Expression::Identifier("x".to_string())),
                BinaryOp::Greater,
                Box::new(Expression::Literal(Literal::Integer(0)))
            )),
            body: Block {
                statements: vec![
                    Statement::Expression(Expression::Literal(Literal::String("positive".to_string())))
                ]
            }
        };

        let display = format!("{}", arm);
        assert!(display.contains("1 if (x > 0) =>"));
    }
}