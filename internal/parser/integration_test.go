package parser

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"kanso/internal/ast"
)

func TestFullLanguageIntegration(t *testing.T) {
	// Test a comprehensive contract that uses all the modernized language features
	source := `// SPDX-License-Identifier: Apache-2.0
/// This is a comprehensive test contract
/** This contract demonstrates all the new language features */
contract ComprehensiveTest {
    use std::evm::{sender, emit};
    use std::address;
    use std::errors;

    #[storage]
    /// Main contract state
    struct State {
        balances: Table<Address, U256>,
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
    fn create(initial_name: String, supply: U256) writes State {
        let mut total = supply;
        let owner = sender();
        
        require!(total > 0, errors::InvalidAmount);
        require!(owner != address::zero(), errors::ZeroAddress);
        
        State.total_supply = total;
        State.name = initial_name;
        State.balances[owner] = total;
        
        emit(Transfer{from: address::zero(), to: owner, amount: total});
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
        let mut from_balance = State.balances[from];
        let mut to_balance = State.balances[to];
        
        require!(from != to, errors::SelfTransfer);
        require!(from_balance >= amount, errors::InsufficientBalance);
        require!(to != address::zero(), errors::ZeroAddress);
        
        from_balance -= amount;
        to_balance += amount;
        
        State.balances[from] = from_balance;
        State.balances[to] = to_balance;
        
        emit(Transfer{from, to, amount});
        
        return true;
    }
    
    fn helper_validate(addr: Address, amount: U256) -> Bool {
        let mut is_valid = true;
        
        require!(addr != address::zero(), errors::ZeroAddress);
        require!(amount > 0, errors::InvalidAmount);
        
        is_valid
    }
}`

	// Parse the contract
	contract, parseErrors, scanErrors := ParseSource("comprehensive_test.ka", source)

	// Verify no parsing or scanning errors
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.Empty(t, scanErrors, "Should have no scan errors")
	assert.NotNil(t, contract, "Contract should be parsed successfully")

	// Verify contract structure
	assert.Equal(t, "ComprehensiveTest", contract.Name.Value)

	// Verify leading comments (3 comments before contract)
	assert.Len(t, contract.LeadingComments, 3, "Should have 3 leading comments")

	comment1, ok1 := contract.LeadingComments[0].(*ast.Comment)
	assert.True(t, ok1, "First comment should be regular comment")
	assert.Contains(t, comment1.Text, "SPDX-License-Identifier")

	docComment1, ok2 := contract.LeadingComments[1].(*ast.DocComment)
	assert.True(t, ok2, "Second comment should be doc comment")
	assert.Contains(t, docComment1.Text, "comprehensive test contract")

	docComment2, ok3 := contract.LeadingComments[2].(*ast.DocComment)
	assert.True(t, ok3, "Third comment should be doc comment")
	assert.Contains(t, docComment2.Text, "new language features")

	// Verify contract items count (3 use statements + 2 structs + 6 functions = 11 items)
	assert.Len(t, contract.Items, 11, "Should have 11 contract items")

	// Verify use statements (first 3 items)
	useStd, ok := contract.Items[0].(*ast.Use)
	assert.True(t, ok, "First item should be use statement")
	assert.Len(t, useStd.Namespaces, 2, "Should have std and evm namespaces")
	assert.Equal(t, "std", useStd.Namespaces[0].Name.Value)
	assert.Equal(t, "evm", useStd.Namespaces[1].Name.Value)
	assert.Len(t, useStd.Imports, 2, "Should import sender and emit")

	// Verify storage struct
	storageStruct, ok := contract.Items[3].(*ast.Struct)
	assert.True(t, ok, "Fourth item should be State struct")
	assert.Equal(t, "State", storageStruct.Name.Value)
	assert.NotNil(t, storageStruct.Attribute, "Should have storage attribute")
	assert.Equal(t, "storage", storageStruct.Attribute.Name)
	assert.Len(t, storageStruct.Items, 3, "State should have 3 fields")

	// Verify event struct
	eventStruct, ok := contract.Items[4].(*ast.Struct)
	assert.True(t, ok, "Fifth item should be Transfer struct")
	assert.Equal(t, "Transfer", eventStruct.Name.Value)
	assert.NotNil(t, eventStruct.Attribute, "Should have event attribute")
	assert.Equal(t, "event", eventStruct.Attribute.Name)

	// Verify constructor function
	createFn, ok := contract.Items[5].(*ast.Function)
	assert.True(t, ok, "Sixth item should be create function")
	assert.Equal(t, "create", createFn.Name.Value)
	assert.NotNil(t, createFn.Attribute, "Should have create attribute")
	assert.Equal(t, "create", createFn.Attribute.Name)
	assert.False(t, createFn.External, "Constructor should not be external")
	assert.Len(t, createFn.Params, 2, "Constructor should have 2 parameters")
	assert.Len(t, createFn.Writes, 1, "Constructor should write to State")
	assert.Equal(t, "State", createFn.Writes[0].Value)

	// Verify constructor body has let mut and require statements
	assert.NotNil(t, createFn.Body, "Constructor should have body")
	bodyItems := createFn.Body.Items
	assert.GreaterOrEqual(t, len(bodyItems), 6, "Constructor should have at least 6 statements")

	// Check first let mut statement
	letMutStmt, ok := bodyItems[0].(*ast.LetStmt)
	assert.True(t, ok, "First statement should be let")
	assert.True(t, letMutStmt.Mut, "Should be let mut")
	assert.Equal(t, "total", letMutStmt.Name.Value)

	// Check regular let statement
	letStmt, ok := bodyItems[1].(*ast.LetStmt)
	assert.True(t, ok, "Second statement should be let")
	assert.False(t, letStmt.Mut, "Should be regular let")
	assert.Equal(t, "owner", letStmt.Name.Value)

	// Check require statements (should have at least 2)
	requireCount := 0
	for _, item := range bodyItems {
		if _, ok := item.(*ast.RequireStmt); ok {
			requireCount++
		}
	}
	assert.GreaterOrEqual(t, requireCount, 2, "Should have at least 2 require statements")

	// Verify external functions
	nameFn, ok := contract.Items[6].(*ast.Function)
	assert.True(t, ok, "Seventh item should be name function")
	assert.Equal(t, "name", nameFn.Name.Value)
	assert.True(t, nameFn.External, "name function should be external")
	assert.NotNil(t, nameFn.Return, "name function should have return type")
	assert.Equal(t, "String", nameFn.Return.Name.Value)
	assert.Len(t, nameFn.Reads, 1, "name function should read from State")
	assert.Equal(t, "State", nameFn.Reads[0].Value)

	transferFn, ok := contract.Items[9].(*ast.Function)
	assert.True(t, ok, "Tenth item should be transfer function")
	assert.Equal(t, "transfer", transferFn.Name.Value)
	assert.True(t, transferFn.External, "transfer function should be external")
	assert.NotNil(t, transferFn.Return, "transfer function should have return type")
	assert.Equal(t, "Bool", transferFn.Return.Name.Value)
	assert.Len(t, transferFn.Writes, 1, "transfer function should write to State")
	assert.Equal(t, "State", transferFn.Writes[0].Value)

	// Verify transfer function body uses let mut
	transferBodyItems := transferFn.Body.Items
	letMutCount := 0
	for _, item := range transferBodyItems {
		if letStmt, ok := item.(*ast.LetStmt); ok && letStmt.Mut {
			letMutCount++
		}
	}
	assert.GreaterOrEqual(t, letMutCount, 2, "transfer function should have at least 2 let mut statements")

	// Verify helper function (not external)
	helperFn, ok := contract.Items[10].(*ast.Function)
	assert.True(t, ok, "Last item should be helper function")
	assert.Equal(t, "helper_validate", helperFn.Name.Value)
	assert.False(t, helperFn.External, "helper function should not be external")

	// Test AST string representation preserves all features
	contractStr := contract.String()

	// Verify leading comments are preserved and come first
	assert.True(t, strings.Index(contractStr, "SPDX-License-Identifier") < strings.Index(contractStr, "contract ComprehensiveTest"),
		"Leading comments should appear before contract declaration")

	// Verify let mut statements are correctly formatted
	assert.Contains(t, contractStr, "let mut total = supply;", "Should contain let mut statement")
	assert.Contains(t, contractStr, "let mut from_balance =", "Should contain let mut in transfer function")

	// Verify require statements are correctly formatted
	assert.Contains(t, contractStr, "require!", "Should contain require statements")

	// Verify external functions are correctly formatted
	assert.Contains(t, contractStr, "ext fn name()", "Should contain external function")
	assert.Contains(t, contractStr, "ext fn transfer(", "Should contain external transfer function")

	// Verify reads/writes clauses are correctly formatted
	assert.Contains(t, contractStr, "reads(State)", "Should contain reads clause")
	assert.Contains(t, contractStr, "writes(State)", "Should contain writes clause")
}

func TestMetadataPreservation(t *testing.T) {
	// Test that the new AST structure preserves metadata correctly
	source := `// Leading comment
contract TestContract {
    #[storage]
    struct State {
        value: U256,
    }
    
    #[create]
    fn create() writes State {
        let mut counter = 0;
        require!(counter >= 0, errors::InvalidValue);
    }
}`

	result := ParseSourceWithMetadata("test.ka", source)
	assert.NotNil(t, result.Contract, "Should parse contract")
	assert.NotNil(t, result.MetadataVisitor, "Should have metadata visitor")

	// Verify metadata is tracked for new AST nodes
	tracker := result.MetadataVisitor.GetTracker()
	assert.NotNil(t, tracker, "Should have node tracker")

	allMetadata := tracker.GetAllMetadata()
	assert.NotEmpty(t, allMetadata, "Should have metadata for nodes")

	// Verify debug info includes new structure
	debugInfo := result.GetDebugInfo()
	assert.Contains(t, debugInfo, "AST Metadata Debug Info", "Should contain debug header")

	// The debug info should contain information about the new structure
	assert.NotEmpty(t, debugInfo, "Debug info should not be empty")
}
