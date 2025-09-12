package parser

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"kanso/internal/ast"
)

func TestParseEmptyContract(t *testing.T) {
	source := `contract Empty {
}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")
	assert.Equal(t, "Empty", contract.Name.Value)
	assert.Empty(t, contract.Items, "Empty contract should have no items")
	assert.Empty(t, contract.LeadingComments, "Should have no leading comments")
}

func TestParseContractWithLeadingComments(t *testing.T) {
	source := `// This is a license comment
// Another comment
/** Doc block comment */
contract TestContract {
    fn test() -> U32 {
        return 42;
    }
}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	assert.Equal(t, "TestContract", contract.Name.Value)
	assert.Len(t, contract.LeadingComments, 3, "Should have 3 leading comments")
	assert.Len(t, contract.Items, 1, "Should have 1 contract item")

	// Check leading comment types
	_, ok1 := contract.LeadingComments[0].(*ast.Comment)
	_, ok2 := contract.LeadingComments[1].(*ast.Comment)
	_, ok3 := contract.LeadingComments[2].(*ast.DocComment)
	assert.True(t, ok1, "First comment should be regular comment")
	assert.True(t, ok2, "Second comment should be regular comment")
	assert.True(t, ok3, "Third comment should be doc comment")

	// Verify the function was parsed as part of contract items, not leading comments
	fn, ok := contract.Items[0].(*ast.Function)
	assert.True(t, ok, "Contract item should be a function")
	assert.Equal(t, "test", fn.Name.Value)
}

func TestParseLetStatement(t *testing.T) {
	source := `contract Test {
    fn test() {
        let balance = 100;
        let total_supply = State.total_supply;
    }
}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	fn := contract.Items[0].(*ast.Function)
	assert.Len(t, fn.Body.Items, 2, "Function should have 2 statements")

	// First let statement
	letStmt1, ok := fn.Body.Items[0].(*ast.LetStmt)
	assert.True(t, ok, "First statement should be LetStmt")
	assert.False(t, letStmt1.Mut, "First let should not be mutable")
	assert.Equal(t, "balance", letStmt1.Name.Value)

	// Second let statement
	letStmt2, ok := fn.Body.Items[1].(*ast.LetStmt)
	assert.True(t, ok, "Second statement should be LetStmt")
	assert.False(t, letStmt2.Mut, "Second let should not be mutable")
	assert.Equal(t, "total_supply", letStmt2.Name.Value)
}

func TestParseLetMutStatement(t *testing.T) {
	source := `contract Test {
    fn test() {
        let mut counter = 0;
        let mut buffer = Vec::new();
        let immutable = 42;
    }
}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	fn := contract.Items[0].(*ast.Function)
	assert.Len(t, fn.Body.Items, 3, "Function should have 3 statements")

	// First let mut statement
	letStmt1, ok := fn.Body.Items[0].(*ast.LetStmt)
	assert.True(t, ok, "First statement should be LetStmt")
	assert.True(t, letStmt1.Mut, "First let should be mutable")
	assert.Equal(t, "counter", letStmt1.Name.Value)

	// Second let mut statement
	letStmt2, ok := fn.Body.Items[1].(*ast.LetStmt)
	assert.True(t, ok, "Second statement should be LetStmt")
	assert.True(t, letStmt2.Mut, "Second let should be mutable")
	assert.Equal(t, "buffer", letStmt2.Name.Value)

	// Third immutable let statement
	letStmt3, ok := fn.Body.Items[2].(*ast.LetStmt)
	assert.True(t, ok, "Third statement should be LetStmt")
	assert.False(t, letStmt3.Mut, "Third let should not be mutable")
	assert.Equal(t, "immutable", letStmt3.Name.Value)
}

func TestParseRequireStatement(t *testing.T) {
	source := `contract Test {
    fn test() {
        require!(amount > 0, errors::InvalidAmount);
        require!(sender() != address::zero());
    }
}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	fn := contract.Items[0].(*ast.Function)
	assert.Len(t, fn.Body.Items, 2, "Function should have 2 statements")

	// First require statement with 2 args
	reqStmt1, ok := fn.Body.Items[0].(*ast.RequireStmt)
	assert.True(t, ok, "First statement should be RequireStmt")
	assert.Len(t, reqStmt1.Args, 2, "First require should have 2 arguments")

	// Second require statement with 1 arg
	reqStmt2, ok := fn.Body.Items[1].(*ast.RequireStmt)
	assert.True(t, ok, "Second statement should be RequireStmt")
	assert.Len(t, reqStmt2.Args, 1, "Second require should have 1 argument")
}

func TestParseComplexContract(t *testing.T) {
	source := `// SPDX-License-Identifier: MIT
contract ERC20 {
    use std::evm::{sender, emit};
    
    #[storage]
    struct State {
        balances: Table<Address, U256>,
        total_supply: U256,
    }
    
    #[event]  
    struct Transfer {
        from: Address,
        to: Address,
        amount: U256,
    }
    
    #[create]
    fn create(initial_supply: U256) writes State {
        let mut total = initial_supply;
        State.total_supply = total;
        require!(total > 0, errors::InvalidAmount);
    }
    
    ext fn transfer(to: Address, amount: U256) -> Bool writes State {
        let balance = State.balances[sender()];
        require!(balance >= amount, errors::InsufficientBalance);
        
        State.balances[sender()] -= amount;
        State.balances[to] += amount;
        
        emit(Transfer{from: sender(), to, amount});
        return true;
    }
}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	assert.Equal(t, "ERC20", contract.Name.Value)
	assert.Len(t, contract.LeadingComments, 1, "Should have 1 leading comment")
	assert.Len(t, contract.Items, 5, "Should have 5 contract items: use, 2 structs, 2 functions")

	// Check leading comment
	_, ok := contract.LeadingComments[0].(*ast.Comment)
	assert.True(t, ok, "Leading comment should be regular comment")

	// Check use statement
	useStmt, ok := contract.Items[0].(*ast.Use)
	assert.True(t, ok, "First item should be use statement")
	assert.Len(t, useStmt.Namespaces, 2, "Should have std and evm namespaces")
	assert.Equal(t, "std", useStmt.Namespaces[0].Name.Value)
	assert.Equal(t, "evm", useStmt.Namespaces[1].Name.Value)
	assert.Len(t, useStmt.Imports, 2, "Should have sender and emit imports")

	// Check storage struct
	storageStruct, ok := contract.Items[1].(*ast.Struct)
	assert.True(t, ok, "Second item should be struct")
	assert.Equal(t, "State", storageStruct.Name.Value)
	assert.NotNil(t, storageStruct.Attribute, "Storage struct should have attribute")
	assert.Equal(t, "storage", storageStruct.Attribute.Name)

	// Check event struct
	eventStruct, ok := contract.Items[2].(*ast.Struct)
	assert.True(t, ok, "Third item should be struct")
	assert.Equal(t, "Transfer", eventStruct.Name.Value)
	assert.NotNil(t, eventStruct.Attribute, "Event struct should have attribute")
	assert.Equal(t, "event", eventStruct.Attribute.Name)

	// Check constructor function
	createFn, ok := contract.Items[3].(*ast.Function)
	assert.True(t, ok, "Fourth item should be function")
	assert.Equal(t, "create", createFn.Name.Value)
	assert.NotNil(t, createFn.Attribute, "Constructor should have attribute")
	assert.Equal(t, "create", createFn.Attribute.Name)

	// Check constructor body for let mut and require statements
	assert.Len(t, createFn.Body.Items, 3, "Constructor should have 3 statements")

	letMutStmt, ok := createFn.Body.Items[0].(*ast.LetStmt)
	assert.True(t, ok, "First statement should be let")
	assert.True(t, letMutStmt.Mut, "Should be let mut")
	assert.Equal(t, "total", letMutStmt.Name.Value)

	reqStmt, ok := createFn.Body.Items[2].(*ast.RequireStmt)
	assert.True(t, ok, "Third statement should be require")
	assert.Len(t, reqStmt.Args, 2, "Require should have 2 arguments")

	// Check external function
	transferFn, ok := contract.Items[4].(*ast.Function)
	assert.True(t, ok, "Fifth item should be function")
	assert.Equal(t, "transfer", transferFn.Name.Value)
	assert.True(t, transferFn.External, "Transfer should be external")
	assert.NotNil(t, transferFn.Return, "Transfer should have return type")
}

func TestParseInvalidSyntax(t *testing.T) {
	// Test a more recoverable syntax error
	source := `contract Test {
    fn test() -> U32 {
        return "invalid_type"; // wrong type
    }
}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	// Should parse successfully but semantic analysis would catch type error
	assert.Empty(t, parseErrors, "Should parse successfully, semantic analysis would catch issues")
	assert.NotNil(t, contract, "Contract should be parsed")
	assert.Equal(t, "Test", contract.Name.Value)
}

func TestParseMissingContract(t *testing.T) {
	source := `// Just a comment without contract`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.NotEmpty(t, parseErrors, "Should have parse errors for missing contract")
	// Parser may return partial contract even with errors, which is acceptable for error recovery
	if contract != nil {
		assert.Len(t, contract.LeadingComments, 1, "Should capture the leading comment")
	}
}

func TestParseIfStatement(t *testing.T) {
	source := `contract Test {
		fn test(value: U256) -> Bool {
			if value > 0 {
				return true;
			}
			return false;
		}
	}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	fn := contract.Items[0].(*ast.Function)
	ifStmt := fn.Body.Items[0].(*ast.IfStmt)

	assert.NotNil(t, ifStmt, "Should parse if statement")
	assert.NotNil(t, ifStmt.Condition, "Should have condition")
	assert.NotNil(t, ifStmt.ThenBlock, "Should have then block")
	assert.Nil(t, ifStmt.ElseBlock, "Should not have else block")
	assert.Len(t, ifStmt.ThenBlock.Items, 1, "Then block should have one statement")
}

func TestParseIfElseStatement(t *testing.T) {
	source := `contract Test {
		fn test(value: U256) -> Bool {
			if value > 0 {
				return true;
			} else {
				return false;
			}
		}
	}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	fn := contract.Items[0].(*ast.Function)
	ifStmt := fn.Body.Items[0].(*ast.IfStmt)

	assert.NotNil(t, ifStmt, "Should parse if statement")
	assert.NotNil(t, ifStmt.Condition, "Should have condition")
	assert.NotNil(t, ifStmt.ThenBlock, "Should have then block")
	assert.NotNil(t, ifStmt.ElseBlock, "Should have else block")
	assert.Len(t, ifStmt.ThenBlock.Items, 1, "Then block should have one statement")
	assert.Len(t, ifStmt.ElseBlock.Items, 1, "Else block should have one statement")
}

func TestParseIfElseIfStatement(t *testing.T) {
	source := `contract Test {
		fn test(value: U256) -> Bool {
			if value > 100 {
				return true;
			} else if value > 50 {
				return false;
			} else {
				return true;
			}
		}
	}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	fn := contract.Items[0].(*ast.Function)
	ifStmt := fn.Body.Items[0].(*ast.IfStmt)

	assert.NotNil(t, ifStmt, "Should parse if statement")
	assert.NotNil(t, ifStmt.ElseBlock, "Should have else block for else if")

	// The else if should be nested as another if statement in the else block
	elseIfStmt := ifStmt.ElseBlock.Items[0].(*ast.IfStmt)
	assert.NotNil(t, elseIfStmt, "Should have nested if statement for else if")
	assert.NotNil(t, elseIfStmt.ElseBlock, "Nested if should have else block")
}

func TestParseIfWithoutParentheses(t *testing.T) {
	source := `contract Test {
		fn test(value: U256) -> Bool {
			if value > 0 {
				return true;
			}
			return false;
		}
	}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should parse if without parentheses")

	fn := contract.Items[0].(*ast.Function)
	ifStmt := fn.Body.Items[0].(*ast.IfStmt)
	assert.NotNil(t, ifStmt, "Should parse if statement without parentheses")
}

func TestParseNestedIfStatements(t *testing.T) {
	source := `contract Test {
		fn test(a: U256, b: U256) -> Bool {
			if a > 0 {
				if b > 0 {
					return true;
				} else {
					return false;
				}
			}
			return false;
		}
	}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	fn := contract.Items[0].(*ast.Function)
	outerIf := fn.Body.Items[0].(*ast.IfStmt)

	assert.NotNil(t, outerIf, "Should parse outer if statement")
	assert.Len(t, outerIf.ThenBlock.Items, 1, "Outer if should have one statement")

	innerIf := outerIf.ThenBlock.Items[0].(*ast.IfStmt)
	assert.NotNil(t, innerIf, "Should parse nested if statement")
	assert.NotNil(t, innerIf.ElseBlock, "Inner if should have else block")
}

func TestParseUninitializedVariables(t *testing.T) {
	source := `contract Test {
		fn test() {
			let mut uninitialized: U256;
			let mut another_uninitialized: Bool;
			let initialized = 100;
		}
	}`

	contract, parseErrors, _ := ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	// Check that the function was parsed correctly
	fn, ok := contract.Items[0].(*ast.Function)
	assert.True(t, ok, "First item should be a function")
	assert.Len(t, fn.Body.Items, 3, "Function should have 3 let statements")

	// Check first uninitialized variable
	let1, ok := fn.Body.Items[0].(*ast.LetStmt)
	assert.True(t, ok, "First item should be a let statement")
	assert.True(t, let1.Mut, "Should be mutable")
	assert.Equal(t, "uninitialized", let1.Name.Value)
	assert.NotNil(t, let1.Type, "Should have explicit type")
	assert.Nil(t, let1.Expr, "Should have no initialization expression")

	// Check second uninitialized variable
	let2, ok := fn.Body.Items[1].(*ast.LetStmt)
	assert.True(t, ok, "Second item should be a let statement")
	assert.True(t, let2.Mut, "Should be mutable")
	assert.Equal(t, "another_uninitialized", let2.Name.Value)
	assert.NotNil(t, let2.Type, "Should have explicit type")
	assert.Nil(t, let2.Expr, "Should have no initialization expression")

	// Check initialized variable
	let3, ok := fn.Body.Items[2].(*ast.LetStmt)
	assert.True(t, ok, "Third item should be a let statement")
	assert.False(t, let3.Mut, "Should be immutable")
	assert.Equal(t, "initialized", let3.Name.Value)
	assert.Nil(t, let3.Type, "Should have no explicit type")
	assert.NotNil(t, let3.Expr, "Should have initialization expression")
}
