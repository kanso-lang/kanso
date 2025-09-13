# Kanso Language

Kanso is a Rust-inspired smart contract programming language with a focus on safety, expressiveness, and developer experience. It features modern syntax, strong typing, and comprehensive semantic analysis for blockchain development.

## Features

- **Rust-inspired Syntax**: Modern, clean syntax familiar to Rust developers
- **Contract Attributes**: Built-in attributes for contract structure (`#[storage]`, `#[event]`, `#[create]`)
- **Semantic Analysis**: Advanced static analysis with reads/writes validation, return value validation, type promotion, and unused function detection
- **Language Server Protocol**: Full IDE support with semantic highlighting, diagnostics, and real-time error detection
- **Mutable Variables**: Support for both immutable (`let`) and mutable (`let mut`) variable declarations
- **External Functions**: Clear distinction between internal and external contract functions

## Quick Start

### Prerequisites

- Go 1.24 or higher
- (Optional) VS Code for IDE support

### Installation

1. Clone the repository:
   ```bash
   git clone git@github.com:kanso-lang/kanso.git
   cd kanso
   ```

2. Build the tools:
   ```bash
   # Build both CLI and LSP server
   make all
   
   # Or build individually
   make kanso      # CLI only
   make kanso-lsp  # LSP server only
   ```

### Usage

#### Compiling Kanso Code

```bash
# Parse and analyze a Kanso contract
./kanso ./examples/erc20.ka
```

#### Language Server Protocol

The Kanso Language Server provides IDE integration:

```bash
# Start the LSP server
./kanso-lsp
```

### IDE Integration

#### VS Code Extension

The VS Code extension is located in `ide/vscode-extension/`:

1. Install dependencies:
   ```bash
   cd ide/vscode-extension
   npm install
   ```

2. Compile the extension:
   ```bash
   npm run package
   ```

3. Install the extension by copying to your VS Code extensions folder or using the VS Code extension development host.

## Language Overview

### Modern Contract Structure

```kanso
// SPDX-License-Identifier: Apache-2.0
contract ERC20 {
    use std::evm::{sender, emit};
    use std::address;
    use std::ascii::{String};
    use std::errors;
    
    #[storage]
    /// Contract state storage
    struct State {
        balances: Slots<Address, U256>,
        total_supply: U256,
        name: String,
    }
    
    #[event]
    struct Transfer {
        from: Address,
        to: Address,
        amount: U256,
    }
    
    #[create]
    /// Contract constructor
    fn create(initial_supply: U256, token_name: String) writes State {
        let owner = sender();
        
        require!(total > 0, errors::InvalidAmount);
        require!(owner != address::zero(), errors::ZeroAddress);
        
        State.total_supply = initial_supply;
        State.name = token_name;
        State.balances[owner] = initial_supply;
        
        emit(Transfer{from: address::zero(), to: owner, amount: initial_supply});
    }
    
    ext fn name() -> String reads State {
        State.name
    }
    
    ext fn totalSupply() -> U256 reads State {
        State.total_supply
    }
    
    ext fn balanceOf(owner: Address) -> U256 reads State {
        State.balances[owner]
    }
    
    ext fn transfer(to: Address, amount: U256) -> Bool writes State {
        let from = sender();
        let mut balance = State.balances[from];
        
        require!(from != to, errors::SelfTransfer);
        require!(balance >= amount, errors::InsufficientBalance);
        
        balance -= amount;
        State.balances[from] = balance;
        State.balances[to] += amount;
        
        emit(Transfer{from, to, amount});
        true
    }
}
```

### Key Language Features

#### Modern Variable Declarations
```kanso
// Immutable variables
let balance = State.balances[owner];
let total_supply = 1000000;

// Mutable variables  
let mut counter = 0;
let mut temp_balance = balance;
counter += 1;
temp_balance -= amount;
```

#### Error Handling with require!
```kanso
// Single condition
require!(amount > 0, errors::InvalidAmount);

// Complex validation
require!(sender() != address::zero(), errors::ZeroAddress);
require!(balance >= amount, errors::InsufficientBalance);
```

#### External vs Internal Functions
```kanso
// External functions (callable from outside)
ext fn transfer(to: Address, amount: U256) -> Bool writes State {
    // Implementation
}

ext fn balanceOf(owner: Address) -> U256 reads State {
    State.balances[owner]
}

// Internal helper functions
fn validate_transfer(from: Address, to: Address, amount: U256) -> Bool {
    // Helper implementation
}
```

#### Reads/Writes Specifications
```kanso
// Read-only functions
ext fn getName() -> String reads State {
    State.name
}

// Functions that modify state
ext fn transfer(to: Address, amount: U256) -> Bool writes State {
    // Modifies State.balances
}

// Functions that read one struct and write another
fn validate(config: Config) reads Config writes AuditLog {
    // Implementation
}
```

## Project Structure

```
kanso/
├── cmd/
│   ├── kanso-cli/          # Main compiler CLI
│   └── kanso-lsp/          # Language Server Protocol implementation
├── internal/
│   ├── ast/                # Abstract Syntax Tree definitions and metadata
│   ├── parser/             # Lexer, parser, and integration tests
│   ├── semantic/           # Semantic analyzer and type checker
│   ├── lsp/                # LSP server implementation
│   ├── builtins/           # Built-in type definitions
│   ├── stdlib/             # Standard library module definitions
│   └── types/              # Type registry and validation
├── ide/
│   ├── vscode-extension/   # VS Code extension
│   └── intellij-plugin/    # IntelliJ IDEA plugin (in development)
├── examples/
│   └── erc20.ka           # Complete ERC20 token contract example
├── Makefile               # Build automation
├── go.mod                 # Go module definition
├── LICENSE                # Apache 2.0 License
├── CLAUDE.md             # Development documentation
└── README.md             # This file
```

## Development

### Building

```bash
# Build all tools
make all

# Clean build artifacts
make clean

# Individual builds
go build -o kanso ./cmd/kanso-cli
go build -o kanso-lsp ./cmd/kanso-lsp
```

### Testing

```bash
# Run all tests (80+ tests across all packages)
go test ./...

# Run specific package tests with verbose output
go test ./internal/parser -v
go test ./internal/semantic -v
go test ./internal/ast -v

# Run tests with coverage
go test ./... -cover

# Run specific test patterns
go test ./internal/parser -run TestParse
go test ./internal/semantic -run TestERC20

# Run specific semantic validation tests
go test ./internal/semantic -run TestReturnValueValidation -v
go test ./internal/semantic -run TestUnusedFunctionDetection -v
go test ./internal/semantic -run TestTypePromotionInFunctionCalls -v
```

### Testing Guidelines

#### Writing Test Functions

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

#### Why ext fn for Tests?

Functions marked with `ext fn` are considered blockchain entry points and are never flagged as unused because they can be called by external transactions. Regular internal functions (`fn`) are only considered used if they're called by other functions within the contract.

#### Test Structure Best Practices

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

#### Semantic Analysis Features

The semantic analyzer validates:

- **Return Value Validation**: Ensures function calls return expected types with proper type promotion
- **Type Promotion**: Validates numeric type promotions (U8 → U16 → U32 → U64 → U128 → U256)
- **Return Statement Validation**: Checks return values match function signatures
- **Unused Function Detection**: Warns about internal functions that are never called
- **Type Compatibility**: Ensures assignments and expressions use compatible types
- **Literal Validation**: Validates numeric, boolean, and string literals
- **Call Path Analysis**: Tracks function calls for reads/writes validation
- **Variable Scoping**: Validates variable declarations and usage within scopes

### Code Organization

- **AST Package** (`internal/ast/`): AST node definitions, metadata system, and string formatting
- **Parser Package** (`internal/parser/`): Lexical analysis, recursive descent parsing, and integration tests
- **Semantic Package** (`internal/semantic/`): Comprehensive type checking, symbol resolution, contract validation, return value validation, type promotion system, and unused function detection
- **LSP Package** (`internal/lsp/`): Language Server Protocol with semantic tokens and diagnostics
- **Builtins Package** (`internal/builtins/`): Built-in type definitions (U8, U16, U32, U64, U128, U256, Bool, Address)
- **Stdlib Package** (`internal/stdlib/`): Standard library module definitions and function signatures
- **CLI Package** (`cmd/kanso-cli/`): Main compiler executable
- **LSP Server** (`cmd/kanso-lsp/`): Language server for IDE integration

## Examples

See the `examples/` directory for complete contract examples:

- **ERC20 Token** (`examples/erc20.ka`): A complete ERC20 token implementation with:
  - Constructor with initial supply and token metadata
  - Transfer functionality with balance validation
  - Allowance system for delegated transfers
  - Event emission for all transfers and approvals
  - Comprehensive error handling

## License

This project is licensed under the Apache License 2.0. See the [LICENSE](LICENSE) file for details.



For detailed development information and architectural decisions, see [CLAUDE.md](CLAUDE.md).