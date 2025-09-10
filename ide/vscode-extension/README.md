# Kanso Language Extension for VS Code

This extension provides rich language support for Kanso smart contract development.

## Features

### Enhanced Syntax Highlighting
- **Semantic Highlighting**: Goes beyond basic syntax coloring to provide contextual highlighting
- **Contract Attributes**: Special highlighting for `#[storage]`, `#[event]`, `#[create]` attributes
- **Symbol Distinction**: Different colors for declarations vs usage, imports vs local symbols

### Intelligent Code Completion
- **Context-Aware Suggestions**: Auto-complete based on current contract structure
- **Kanso Keywords**: Snippets for `let`, `let mut`, `fn`, `ext fn`, `require!`
- **Contract Constructs**: Templates for common patterns like storage structs and event definitions
- **Field Access**: Auto-complete struct fields when accessing `State.`
- **Function Suggestions**: Distinguish between external and internal functions

### Real-time Error Detection
- **Semantic Analysis**: Catch errors beyond basic syntax issues
- **Undefined References**: Detect undefined functions, variables, and imports
- **Type Validation**: Smart contract specific constraints and validations
- **Import Validation**: Verify imported symbols exist in referenced modules

### Configuration Options

Configure the extension through VS Code settings:

```json
{
  "kanso.server.path": "kanso-lsp",
  "kanso.semanticHighlighting.enabled": true,
  "kanso.completion.enabled": true,
  "kanso.diagnostics.enabled": true
}
```

### Commands

- **Kanso: Restart Language Server** - Restart the language server for development
- **Kanso: Show Language Server Output** - View language server logs for debugging

## Installation

1. Install this VS Code extension
2. Open a `.ka` file to activate the language support

The Kanso Language Server is automatically built and packaged with the extension - no separate installation required!

## Requirements

- VS Code 1.100.0 or higher
- Go 1.24+ (only needed if building from source)

## Example

```kanso
contract ERC20 {
  use std::evm::{sender, emit};
  
  #[storage]
  struct State {
    balances: Slots<Address, U256>
  }
  
  #[event]
  struct Transfer {
    from: Address,
    to: Address, 
    value: U256
  }
  
  ext fn transfer(to: Address, amount: U256) -> Bool {
    let balance = State.balances[sender()];
    require!(amount <= balance, "Insufficient balance");
    
    State.balances[sender()] -= amount;
    State.balances[to] += amount;
    
    emit(Transfer{from: sender(), to, value: amount});
    true
  }
}
```

With this extension, you'll get:
- Red underlines for semantic errors
- Auto-complete when typing `State.` shows `balances`
- Distinct highlighting for `#[storage]`, `#[event]`, function names
- Intelligent suggestions for Kanso constructs

## Development

```bash
# Install dependencies
npm install

# Build LSP server and compile extension
npm run compile

# Package extension for distribution
npm run package

# Clean build artifacts
npm run clean
```

The build process automatically:
1. Compiles the `kanso-lsp` binary from the parent project
2. Copies it into the extension's `bin/` directory  
3. Bundles the TypeScript extension code and all dependencies into a single optimized file