# VS Code Extension Tests

This directory contains automated tests for the Kanso VS Code extension.

## Test Structure

- `suite/extension.test.ts` - Main extension functionality tests
- `suite/syntax.test.ts` - Syntax highlighting and grammar tests  
- `suite/index.ts` - Test runner setup
- `runTest.ts` - VS Code test environment setup

## Running Tests

### Local Testing

```bash
# Install dependencies
npm install

# Compile extension and tests
npm run pretest

# Run tests (requires X11 display on Linux)
npm test
```

### Debug Tests in VS Code

1. Open VS Code in the extension directory
2. Go to Run and Debug (Ctrl+Shift+D)
3. Select "Extension Tests" configuration
4. Press F5 to run tests in debug mode

## Test Categories

### Extension Tests (`extension.test.ts`)

- **Extension Loading**: Verifies the extension activates correctly
- **Language Registration**: Confirms Kanso language is registered for `.ka` files
- **Language Server Features**: Tests semantic highlighting, diagnostics, and completion
- **Error Detection**: Validates real-time syntax error reporting
- **Struct Field Completion**: Tests context-aware autocompletion

### Syntax Tests (`syntax.test.ts`)

- **Grammar Validation**: Verifies the TextMate grammar is valid JSON
- **Keyword Patterns**: Tests recognition of Kanso keywords (`fn`, `ext`, `contract`, etc.)
- **Type Patterns**: Validates primitive and generic type highlighting
- **Attribute Patterns**: Tests `#[storage]`, `#[event]`, `#[create]` attributes
- **Semantic Token Generation**: Verifies rich syntax highlighting

## CI Integration

The tests run automatically in GitHub Actions on:
- Push to main branch (when extension files change)
- Pull requests affecting the extension
- Changes to Go source code (since LSP depends on it)

The CI workflow:
1. Builds the Kanso LSP server
2. Installs extension dependencies 
3. Compiles the extension
4. Runs Go tests
5. Runs VS Code extension tests in headless mode
6. Packages the extension as artifact

## Test Dependencies

- `@vscode/test-electron`: VS Code test runner
- `mocha`: JavaScript test framework
- `@types/mocha`: TypeScript definitions
- `xvfb`: Virtual display for headless testing (CI only)

## Writing New Tests

When adding new language features:

1. Update `syntax.test.ts` if adding new syntax patterns
2. Add functional tests to `extension.test.ts` for LSP features
3. Ensure tests are deterministic and clean up after themselves
4. Use appropriate timeouts for LSP operations (usually 2-3 seconds)

## Common Issues

- **Timeout Errors**: Language server needs time to initialize and process files
- **Display Issues**: Tests require a display environment (use xvfb in headless environments)
- **Extension Conflicts**: Tests run with `--disable-extensions` to avoid interference
- **File Cleanup**: Always close test documents to prevent resource leaks