# Kanso Language

Kanso is a Move-inspired smart contract programming language with a focus on safety, expressiveness, and developer experience. It features resource-oriented programming, strong typing, and formal verification capabilities.

## Features

- **Move-inspired Syntax**: Familiar syntax for developers coming from Move
- **Strong Type System**: Generic types, ownership tracking, and memory safety
- **Contract Attributes**: Built-in attributes for contract structure (`#[contract]`, `#[storage]`, `#[event]`, `#[create]`)
- **Language Server Protocol**: Full IDE support with semantic highlighting, diagnostics, and autocomplete (in progress)

## Quick Start

### Prerequisites

- Go 1.24 or higher
- (Optional) VS Code or IntelliJ IDEA for IDE support

### Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd kanso-lang
   ```

2. Build the tools:
   ```bash
   # Build both CLI and LSP server
   go build -o kanso ./cmd/kanso-cli
   go build -o kanso-lsp ./cmd/kanso-lsp
   ```

### Usage

#### Compiling Kanso Code

```bash
# Compile and print AST
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

The VS Code extension is located in `ide/vscode-extension/kanso/`:

1. Install dependencies:
   ```bash
   cd ide/vscode-extension/kanso
   npm install
   ```

2. Compile the extension:
   ```bash
   npm run compile
   ```

3. Install the extension by copying to your VS Code extensions folder or using the VS Code extension development host.

#### IntelliJ Plugin

The IntelliJ plugin is located in `ide/intellij-plugin/kanso/`.

## Language Overview

### Contract Structure

```kanso
#[contract]
module MyContract {
    use std::ascii::{String};
    
    #[storage]
    struct State {
        value: u64,
        owner: address,
    }
    
    #[create]
    fun create(initial_value: u64) writes State {
        move_to<State>(State {
            value: initial_value,
            owner: sender(),
        });
    }
    
    public fun get_value(): u64 reads State {
        borrow_global<State>().value
    }
}
```

### Key Language Features

#### Generic Types
```kanso
struct Table<K, V> {
    data: vector<Entry<K, V>>,
}

fun borrow_mut<T>(table: &mut Table<address, T>, key: &address): &mut T
```

#### Resource Management
```kanso
// Move resources with move_to and borrow_global
move_to<State>(state_instance);
let state_ref = borrow_global<State>();
let state_mut_ref = borrow_global_mut<State>();
```

#### Function Specifications
```kanso
// Specify what global state the function reads/writes
public fun transfer(to: address, amount: u256): bool writes State {
    // Function implementation
}

public fun balance(): u256 reads State {
    // Read-only function
}
```

## Project Structure

```
kanso-lang/
├── cmd/
│   ├── kanso-cli/          # Main compiler CLI
│   └── kanso-lsp/          # Language Server Protocol implementation
├── internal/
│   ├── ast/                # Abstract Syntax Tree definitions
│   ├── parser/             # Lexer and parser implementation
│   └── lsp/                # LSP server implementation
├── ide/
│   ├── vscode-extension/   # VS Code extension
│   └── intellij-plugin/    # IntelliJ IDEA plugin
├── examples/
│   └── erc20.ka           # Example ERC20 token contract
├── go.mod                 # Go module definition
├── LICENSE                # Apache 2.0 License
├── CLAUDE.md             # Development documentation
└── README.md             # This file
```

## Development

### Building

```bash
# Build CLI
go build -o kanso ./cmd/kanso-cli

# Build LSP server
go build -o kanso-lsp ./cmd/kanso-lsp
```

### Testing

```bash
# Run all tests
go test ./...

# Run specific package tests
go test ./internal/parser -v
go test ./internal/lsp -v
```

### Code Organization

- **AST Package** (`internal/ast/`): Defines the Abstract Syntax Tree nodes for all language constructs
- **Parser Package** (`internal/parser/`): Implements lexical analysis and recursive descent parsing
- **LSP Package** (`internal/lsp/`): Provides Language Server Protocol implementation with semantic tokens
- **CLI Package** (`cmd/kanso-cli/`): Main compiler executable
- **LSP Server** (`cmd/kanso-lsp/`): Language server executable for IDE integration

## Language Reference

### Types

- **Primitive Types**: `bool`, `u8`, `u64`, `u128`, `u256`, `address`
- **String Types**: `String` (from `std::ascii`)
- **Generic Types**: `Table<K, V>`, `vector<T>`
- **Reference Types**: `&T`, `&mut T`

### Attributes

- `#[contract]`: Marks a module as a smart contract
- `#[storage]`: Marks a struct as contract storage
- `#[event]`: Marks a struct as an event that can be emitted
- `#[create]`: Marks a function as the contract constructor

### Built-in Functions

- `move_to<T>(resource)`: Moves a resource to global storage
- `borrow_global<T>()`: Immutably borrows from global storage
- `borrow_global_mut<T>()`: Mutably borrows from global storage
- `sender()`: Returns the address of the transaction sender
- `emit(event)`: Emits an event

## Examples

See the `examples/` directory for complete contract examples:

- **ERC20 Token** (`examples/erc20.ka`): A complete ERC20 token implementation with transfers, allowances, and events

## License

This project is licensed under the Apache License 2.0. See the [LICENSE](LICENSE) file for details.

## IDE Support Status

- ✅ **Syntax Highlighting**: Complete syntax highlighting for all language constructs
- ✅ **Semantic Tokens**: Advanced semantic highlighting with proper categorization
- ✅ **Error Diagnostics**: Real-time error detection and reporting
- ⚠️ **Autocomplete**: Basic completion support (in development)
- ⚠️ **Go to Definition**: Navigation support (in development)
- ⚠️ **Hover Information**: Type and documentation display (in development)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## Roadmap

- [ ] Enhanced type checking and inference
- [ ] Formal verification integration
- [ ] Bytecode generation
- [ ] Runtime environment
- [ ] Package manager
- [ ] Testing framework
- [ ] Documentation generator

---

For detailed development information, see [CLAUDE.md](CLAUDE.md).