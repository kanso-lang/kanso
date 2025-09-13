# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Kanso is a Rust-inspired smart contract programming language with modern syntax and comprehensive static analysis. The project consists of:

- **kanso-cli**: Command-line compiler that parses and analyzes `.ka` files
- **kanso-lsp**: Language Server Protocol implementation for IDE integration
- **vscode-extension**: VS Code extension providing syntax highlighting and LSP integration
- **intellij-plugin**: IntelliJ plugin (in development)

The language syntax is Rust-like with smart contract-specific features including `#[storage]`, `#[event]`, and `#[create]` attributes. Key modernizations include `contract` declarations (not modules), `let mut` variables, `require!` error handling, `ext fn` external functions, and uppercase builtin types.

## Build Commands

```bash
# Build both CLI and LSP binaries
make all

# Build CLI only
make kanso

# Build LSP only  
make kanso-lsp

# Clean binaries
make clean
```

## Testing

```bash
# Run all tests (80+ tests across all packages)
go test ./...

# Run specific package tests
go test ./internal/parser -v
go test ./internal/semantic -v
go test ./internal/ast -v

# Run integration tests
go test ./internal/parser -run TestFullLanguageIntegration

# Run tests with coverage
go test ./... -cover

# Run specific semantic test suites
go test ./internal/semantic -run TestReturnValueValidation -v
go test ./internal/semantic -run TestUnusedFunctionDetection -v
go test ./internal/semantic -run TestTypePromotionInFunctionCalls -v
```

## Architecture

### Core Components

**AST Package (`internal/ast/`)**
- Defines all AST node types with a unified `Node` interface and metadata system
- Each node implements `NodePos()`, `NodeEndPos()`, `NodeType()`, and `String()`
- Core nodes: `Contract` (with `LeadingComments`, `Name`, `Items`), `Function` (with `External` field), `LetStmt` (with `Mut` field), `RequireStmt`
- Auto-generated string enums for node types using `go generate`
- Comprehensive printer system for AST-to-source conversion
- Metadata visitor system for tracking compilation information

**Parser Package (`internal/parser/`)**
- `scanner.go`: Tokenizes Kanso source code with modern keywords (`fn`, `ext`, `require`, `let mut`)
- `parser.go`: Main parsing logic using recursive descent, handles leading comments and contract structure
- `parser_pratt.go`: Pratt parser for expressions with precedence handling
- `parser_function.go`: Function parsing with external flag and parameter handling
- `parser_struct.go`, `parser_use.go`: Specialized parsers for structs and imports
- Entry point: `ParseSource(filename, source)` returns `(contract, parseErrors, scanErrors)`
- Comprehensive test suite with 17 parser-specific tests

**Semantic Package (`internal/semantic/`)**
- `analyzer.go`: Comprehensive semantic analysis with type checking and validation
- `analyzer_expression.go`: Expression type inference and function call validation
- `analyzer_declaration.go`: Variable declaration and assignment validation
- Validates reads/writes clauses, constructor requirements, and function calls
- Return value validation ensures function calls return expected types
- Type promotion system supports numeric type widening (U8 → U16 → U32 → U64 → U128 → U256)
- Unused function detection warns about internal functions that are never called
- Return statement validation checks function return values match signatures
- Processes `use` statements and integrates with standard library definitions
- Handles `LetStmt` with mutability tracking and `RequireStmt` validation
- 50+ semantic tests covering all validation scenarios including edge cases
- Full integration with modernized AST structure

**LSP Package (`internal/lsp/`)**
- `handler.go`: Implements LSP server handlers for VS Code integration
- `diagnostics.go`: Provides syntax and semantic error diagnostics
- `semantic.go`: Semantic token highlighting for all language constructs
- Updated for new contract structure with leading comments
- Thread-safe with mutex protection for concurrent access

**Builtins Package (`internal/builtins/`)**
- Defines uppercase builtin types: `U8`, `U16`, `U32`, `U64`, `U128`, `U256`, `Bool`, `Address`
- Updated from old lowercase types to match modern language

**Stdlib Package (`internal/stdlib/`)**
- Standard library module definitions and function signatures
- Core modules: `std::evm`, `std::address`, `std::ascii`, `std::errors`
- Function definitions for `sender()`, `emit()`, `address::zero()`, etc.

### Binary Targets

**CLI (`cmd/kanso-cli/main.go`)**
- Parses `.ka` files and outputs formatted AST with semantic analysis
- Provides detailed error messages with source context
- Usage: `./kanso <file.ka>`
- Supports the complete modernized language

**LSP Server (`cmd/kanso-lsp/main.go`)**
- Runs as background process for IDE integration
- Communicates via JSON-RPC protocol
- Provides real-time error detection and semantic highlighting

## Key Dependencies

- `github.com/tliron/glsp`: LSP server implementation
- `github.com/fatih/color`: Terminal color output  
- `github.com/stretchr/testify`: Testing framework (used in 80+ tests)

## Development Workflow

1. **Parser-First Development**: Most language changes start with parser updates
2. **AST Evolution**: AST nodes should be added to both type definitions and `Node` interface implementations
3. **Semantic Integration**: New language features require semantic analyzer updates
4. **LSP Updates**: IDE integration requires updating semantic token types and diagnostics
5. **Comprehensive Testing**: All changes must include parser, AST, and semantic tests
6. **Documentation**: Update both README.md and CLAUDE.md for language changes

## Language Features (Modernized)

### Current Language Syntax

Kanso supports modern Rust-like syntax:

```kanso
// SPDX-License-Identifier: Apache-2.0
contract ERC20 {
    use std::evm::{sender, emit};
    use std::address;
    use std::errors;
    
    #[storage]
    struct State {
        balances: Slots<Address, U256>,
        total_supply: U256,
    }
    
    #[event]
    struct Transfer {
        from: Address,
        to: Address, 
        amount: U256,
    }
    
    #[create]
    fn create(supply: U256) writes State {
        let mut total = supply;
        let owner = sender();
        
        require!(total > 0, errors::InvalidAmount);
        
        State.total_supply = total;
        State.balances[owner] = total;
    }
    
    ext fn transfer(to: Address, amount: U256) -> Bool writes State {
        let from = sender();
        let mut balance = State.balances[from];
        
        require!(balance >= amount, errors::InsufficientBalance);
        
        balance -= amount;
        State.balances[from] = balance;
        State.balances[to] += amount;
        
        emit(Transfer{from, to, amount});
        true
    }
}
```

### Key Language Elements

- **Contract Structure**: `contract Name { ... }` (not module-based)
- **Leading Comments**: Comments before contract declaration are preserved
- **Variable Declarations**: `let` (immutable) and `let mut` (mutable)
- **External Functions**: `ext fn` for externally callable functions
- **Error Handling**: `require!(condition, error)` for validation
- **Type System**: Uppercase builtin types (`U256`, `Bool`, `Address`)
- **Import System**: `use std::module::{items}` syntax
- **Attributes**: `#[storage]`, `#[event]`, `#[create]`
- **Reads/Writes**: Functions specify storage access patterns

### AST Structure

- `Contract`: Contains `LeadingComments` and `Items` (not `ContractItems`)
- `Function`: Has `External` field (not `Public`) for `ext fn`
- `LetStmt`: Has `Mut` field for `let mut` support
- `RequireStmt`: Renamed from `AssertStmt` for `require!` syntax

## Test Coverage

The project maintains comprehensive test coverage with 80+ tests across all packages:

- **Parser Tests**: 17 tests covering all parsing scenarios
- **AST Tests**: 15 tests for node creation and string representation  
- **Semantic Tests**: 50+ tests for comprehensive type checking and validation
- **LSP Tests**: Integration tests for Language Server Protocol
- **Integration Tests**: End-to-end validation of complete language features

Key test files:
- `internal/parser/parser_test.go`: Core parsing functionality
- `internal/parser/integration_test.go`: Comprehensive language tests
- `internal/ast/printer_test.go`: AST string representation tests
- `internal/semantic/analyzer_test.go`: Core semantic analysis tests
- `internal/semantic/return_value_test.go`: Function call return value validation (11 tests)
- `internal/semantic/return_type_validation_test.go`: Return statement type validation (12 tests)
- `internal/semantic/type_promotion_test.go`: Numeric type promotion validation (13 tests)
- `internal/semantic/unused_function_test.go`: Unused function detection (7 tests)
- `internal/semantic/literal_validation_test.go`: Literal value validation (5 tests)

## Testing Guidelines

### Writing Test Functions

**IMPORTANT**: Test functions should always use the `ext fn` modifier to avoid unused function warnings:

```kanso
// CORRECT: Use ext fn for test functions
contract Test {
    ext fn test_something() {
        let x = 42;
    }
}

// INCORRECT: Regular fn will trigger unused function warning
contract Test {
    fn test_something() {  // Warning: function 'test_something' is defined but never used
        let x = 42;
    }
}
```

### Why `ext fn` for Tests?

Functions marked with `ext fn` are considered blockchain entry points and are never flagged as unused because they can be called by external transactions. Regular internal functions (`fn`) are only considered used if they're called by other functions within the contract.

### Test Structure Best Practices

1. **Use External Functions**: Always mark test functions as `ext fn`
2. **Descriptive Names**: Use clear, descriptive function names for tests
3. **Type Annotations**: Include explicit type annotations to test type validation
4. **Error Cases**: Test both valid and invalid scenarios
5. **Multiple Contracts**: Use separate contracts for different test scenarios

```kanso
contract ValidTypeTest {
    ext fn test_valid_assignment() {
        let x: U256 = 42;  // Valid: literal promotes to U256
    }
}

contract InvalidTypeTest {
    ext fn test_invalid_assignment() {
        let x: U8 = 300;   // Invalid: exceeds U8 range
    }
}
```

### Semantic Analysis Test Coverage

The semantic analyzer validates:

- **Return Value Validation**: Ensures function calls return expected types with proper type promotion
- **Type Promotion**: Validates numeric type promotions (U8 → U16 → U32 → U64 → U128 → U256)
- **Return Statement Validation**: Checks return values match function signatures
- **Unused Function Detection**: Warns about internal functions that are never called
- **Type Compatibility**: Ensures assignments and expressions use compatible types
- **Literal Validation**: Validates numeric, boolean, and string literals
- **Call Path Analysis**: Tracks function calls for reads/writes validation
- **Variable Scoping**: Validates variable declarations and usage within scopes

### Running Tests

```bash
# Run all tests with verbose output
go test ./... -v

# Run specific semantic tests
go test ./internal/semantic -v

# Run tests with coverage
go test ./... -cover

# Run specific test function
go test ./internal/semantic -run TestReturnValueValidation -v
go test ./internal/semantic -run TestUnusedFunctionDetection -v
go test ./internal/semantic -run TestTypePromotionInFunctionCalls -v

# Run parser integration tests
go test ./internal/parser -run TestFullLanguageIntegration -v
```

## IDE Integration Status

- **Syntax Highlighting**: Complete for all modern language constructs
- **Semantic Tokens**: Advanced highlighting with proper categorization
- **Error Diagnostics**: Real-time error detection and reporting
- **Parse Error Recovery**: Graceful handling of syntax errors
- **Type Validation**: Built-in and user-defined type checking with promotion
- **Return Value Validation**: Function call return type checking
- **Unused Function Detection**: Warning for internal functions never called
- **Contract Analysis**: Reads/writes validation and constructor analysis

## Development Guidelines

### Adding New Language Features

1. **Update Scanner**: Add new keywords/tokens in `internal/parser/scanner.go`
2. **Update AST**: Define new node types in `internal/ast/contract.go`
3. **Update Parser**: Add parsing logic in appropriate `internal/parser/parser_*.go` files
4. **Update Semantic**: Add validation in `internal/semantic/analyzer.go`
5. **Update LSP**: Add semantic highlighting in `internal/lsp/semantic.go`
6. **Add Tests**: Create comprehensive tests for all new functionality
7. **Update Documentation**: Update both README.md and CLAUDE.md

### Code Quality Standards

- All public types and functions must have documentation comments
- Comments should explain "why" rather than "what" 
- New features require comprehensive test coverage
- AST nodes must implement the `Node` interface correctly
- Parser changes must handle error recovery gracefully
- Semantic analysis must provide helpful error messages

### Important Implementation Notes

- The `External` field in `Function` represents `ext fn` syntax
- The `Mut` field in `LetStmt` represents `let mut` declarations
- `Contract.LeadingComments` preserves comments before contract declaration
- `RequireStmt` handles `require!(condition, error)` syntax
- Builtin types are uppercase: `U32`, `Bool`, `Address`
- Standard library uses `std::module::{items}` import syntax

## Examples

The `examples/erc20.ka` file demonstrates all modern language features:
- Leading comments with licensing
- Modern import statements
- Storage and event structs
- Constructor with error handling
- External functions with reads/writes clauses
- Mutable variable usage
- Comprehensive error validation

---

This documentation reflects the current modernized state of the Kanso language as of the AST evolution from Move-like to Rust-like syntax.