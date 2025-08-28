# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Kanso is a programming language for smart contracts inspired by Move. The project consists of:

- **kanso-cli**: Command-line compiler that parses and processes `.ka` files
- **kanso-lsp**: Language Server Protocol implementation for IDE integration
- **vscode-extension**: VS Code extension providing syntax highlighting and LSP integration
- **intellij-plugin**: IntelliJ plugin (in development)

The language syntax is similar to Move/Rust with contract-specific features like `#[contract]`, `#[storage]`, `#[event]`, and `#[create]` attributes.

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
# Run all tests
go test ./...

# Run specific package tests
go test ./internal/parser
go test ./internal/lsp

# Run specific test files
go test ./internal/parser -run TestPratt
go test ./internal/parser/scanner_test.go
```

## Architecture

### Core Components

**AST Package (`internal/ast/`)**
- Defines all AST node types with a unified `Node` interface
- Each node implements `NodePos()`, `NodeEndPos()`, `NodeType()`, and `String()`
- Includes contract-specific nodes: `Contract`, `Struct`, `Function`, etc.
- Auto-generated string enums for node types using `go generate`

**Parser Package (`internal/parser/`)**
- `scanner.go`: Tokenizes Kanso source code
- `parser.go`: Main parsing logic using recursive descent
- `parser_pratt.go`: Pratt parser for expressions with precedence handling
- `parser_function.go`, `parser_struct.go`, `parser_use.go`: Specialized parsers
- Entry point: `ParseSource(filename, source)` returns `(contract, parseErrors, scanErrors)`

**LSP Package (`internal/lsp/`)**
- `handler.go`: Implements LSP server handlers for VS Code integration
- `diagnostics.go`: Provides syntax error diagnostics
- `semantic.go`: Semantic token highlighting
- Thread-safe with mutex protection for concurrent access

### Binary Targets

**CLI (`cmd/kanso-cli/main.go`)**
- Parses `.ka` files and outputs formatted AST
- Provides detailed error messages with source context
- Usage: `./kanso <file.ka>`

**LSP Server (`cmd/kanso-lsp/main.go`)**
- Runs as background process for IDE integration
- Communicates via JSON-RPC protocol

## Key Dependencies

- `github.com/tliron/glsp`: LSP server implementation
- `github.com/fatih/color`: Terminal color output  
- `github.com/stretchr/testify`: Testing framework

## Development Workflow

1. The parser is the core component - most language changes start here
2. AST nodes should be added to both the type definitions and the `Node` interface implementations
3. LSP integration requires updating semantic token types and diagnostics
4. Test files use standard Go testing with `_test.go` suffix
5. Examples are in `examples/erc20.ka` showing contract syntax

## Language Features

Kanso supports:
- Contract modules with `#[contract]` attribute
- Storage structs with `#[storage]` 
- Events with `#[event]`
- Constructor functions with `#[create]`
- Move-like syntax for functions, structs, and expressions
- Type system with generics (`Table<address, u256>`)
- Import system with `use` statements