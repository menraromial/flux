# Flux Programming Language

A modern programming language designed to combine the simplicity of Python, the concurrency of Go, the safety of Rust, and the robustness of Java.

## Project Structure

```
flux/
├── Cargo.toml              # Project configuration and dependencies
├── README.md               # This file
├── src/
│   ├── lib.rs             # Library root module
│   ├── main.rs            # CLI application entry point
│   ├── error.rs           # Error types and handling
│   ├── position.rs        # Source code position tracking
│   ├── lexer/             # Lexical analysis
│   │   ├── mod.rs         # Lexer trait and implementation
│   │   └── token.rs       # Token definitions
│   ├── parser/            # Syntax analysis
│   │   ├── mod.rs         # Parser trait and implementation
│   │   └── ast.rs         # Abstract Syntax Tree definitions
│   ├── semantic/          # Semantic analysis
│   │   ├── mod.rs         # Semantic analyzer trait and implementation
│   │   ├── symbol_table.rs # Symbol table for name resolution
│   │   └── type_checker.rs # Type checking and inference
│   ├── codegen/           # Code generation
│   │   └── mod.rs         # LLVM-based code generator
│   └── runtime/           # Runtime system
│       ├── mod.rs         # Runtime trait and implementation
│       ├── gc.rs          # Garbage collection
│       └── concurrency.rs # Goroutines and scheduling
└── tests/
    └── integration_test.rs # Integration tests
```

## Core Components

### 1. Lexer (`src/lexer/`)
- **Purpose**: Tokenizes Flux source code into meaningful symbols
- **Key Files**:
  - `mod.rs`: `Lexer` trait and `FluxLexer` implementation
  - `token.rs`: Token type definitions for all language constructs
- **Features**: Handles keywords, operators, literals, identifiers, and comments

### 2. Parser (`src/parser/`)
- **Purpose**: Builds Abstract Syntax Trees from token streams
- **Key Files**:
  - `mod.rs`: `Parser` trait and recursive descent parser implementation
  - `ast.rs`: Complete AST node definitions for all language constructs
- **Features**: Recursive descent parsing with error recovery

### 3. Semantic Analyzer (`src/semantic/`)
- **Purpose**: Performs type checking, name resolution, and semantic validation
- **Key Files**:
  - `mod.rs`: `SemanticAnalyzer` trait and main analysis logic
  - `symbol_table.rs`: Hierarchical symbol table for scope management
  - `type_checker.rs`: Type inference and checking algorithms
- **Features**: Type inference, name resolution, semantic validation

### 4. Code Generator (`src/codegen/`)
- **Purpose**: Translates typed AST to executable code
- **Key Files**:
  - `mod.rs`: `CodeGenerator` trait with LLVM backend (optional)
- **Features**: LLVM-based native code generation (requires `llvm` feature)

### 5. Runtime System (`src/runtime/`)
- **Purpose**: Provides runtime services for Flux programs
- **Key Files**:
  - `mod.rs`: `Runtime` trait and main runtime implementation
  - `gc.rs`: Mark-and-sweep garbage collector
  - `concurrency.rs`: Goroutine scheduler and async runtime
- **Features**: Garbage collection, goroutines, channels, async/await

### 6. Error Handling (`src/error.rs`)
- **Purpose**: Comprehensive error types for all compilation phases
- **Features**: Detailed error messages with source location information

### 7. Position Tracking (`src/position.rs`)
- **Purpose**: Track source code positions for error reporting
- **Features**: Line/column tracking, span management

## Building and Testing

### Prerequisites
- Rust 1.70+ (2021 edition)
- Optional: LLVM 17 for code generation (enable with `--features llvm`)

### Commands

```bash
# Check compilation
cargo check

# Build the project
cargo build

# Run tests
cargo test

# Run the CLI
cargo run -- <command>

# Available CLI commands:
cargo run -- build    # Compile Flux source files
cargo run -- run      # Compile and run Flux program  
cargo run -- test     # Run tests
cargo run -- fmt      # Format source code
cargo run -- lint     # Lint source code
```

### Features

- **Default**: Core compiler without LLVM code generation
- **LLVM**: Enable LLVM-based code generation (`cargo build --features llvm`)

## Current Status

This implementation provides the foundational architecture and interfaces for the Flux language compiler. The current version includes:

✅ **Completed**:
- Project structure and build system
- Core trait interfaces for all compiler phases
- Basic lexer with token recognition
- Recursive descent parser with AST generation
- Symbol table and type checker foundations
- Error handling system with detailed error types
- Position tracking for source locations
- Runtime system architecture
- Garbage collector implementation
- Concurrency primitives (goroutines, channels)
- Integration tests

🚧 **In Progress**:
- Complete lexer implementation (all tokens and edge cases)
- Full parser implementation (all language constructs)
- Complete type system with inference
- LLVM code generation
- Standard library modules
- Package management system

## Architecture Principles

1. **Modular Design**: Clear separation between compilation phases
2. **Trait-Based**: Extensible interfaces for all major components
3. **Error Recovery**: Robust error handling throughout the pipeline
4. **Performance**: Designed for fast compilation and efficient runtime
5. **Safety**: Memory safety and type safety by design
6. **Concurrency**: Built-in support for concurrent programming

## Next Steps

The next phase of development will focus on implementing the detailed functionality for each component, starting with:

1. Complete lexer implementation (Task 2.1-2.3)
2. Full parser implementation (Task 3.1-3.4)
3. Type system implementation (Task 4.1-4.3)
4. LLVM code generation (Task 5.1-5.3)

See `tasks.md` in the `.kiro/specs/flux-language-implementation/` directory for the complete implementation plan.