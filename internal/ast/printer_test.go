package ast

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestContractString(t *testing.T) {
	contract := &Contract{
		Name: Ident{Value: "TestContract"},
		Items: []ContractItem{
			&Function{
				Name: Ident{Value: "test"},
				Body: &FunctionBlock{},
			},
		},
		LeadingComments: []ContractItem{},
	}

	expected := "contract TestContract {\n  fn test() {\n   }\n  \n}"
	assert.Equal(t, expected, contract.String())
}

func TestContractStringWithLeadingComments(t *testing.T) {
	contract := &Contract{
		Name: Ident{Value: "TestContract"},
		Items: []ContractItem{
			&Function{
				Name: Ident{Value: "test"},
				Body: &FunctionBlock{},
			},
		},
		LeadingComments: []ContractItem{
			&Comment{Text: "// License comment"},
			&DocComment{Text: "/// Documentation"},
		},
	}

	result := contract.String()

	// Should start with leading comments
	assert.Contains(t, result, "// License comment")
	assert.Contains(t, result, "/// Documentation")
	assert.Contains(t, result, "contract TestContract {")

	// Comments should appear before contract declaration
	licensePos := findSubstring(result, "// License comment")
	contractPos := findSubstring(result, "contract TestContract")
	assert.True(t, licensePos < contractPos, "License comment should appear before contract declaration")
}

func TestLetStmtString(t *testing.T) {
	// Test immutable let statement
	letStmt := &LetStmt{
		Name: Ident{Value: "balance"},
		Expr: &LiteralExpr{Value: "100"},
		Mut:  false,
	}

	expected := "let balance = 100;"
	assert.Equal(t, expected, letStmt.String())
}

func TestLetMutStmtString(t *testing.T) {
	// Test mutable let statement
	letMutStmt := &LetStmt{
		Name: Ident{Value: "counter"},
		Expr: &LiteralExpr{Value: "0"},
		Mut:  true,
	}

	expected := "let mut counter = 0;"
	assert.Equal(t, expected, letMutStmt.String())
}

func TestRequireStmtString(t *testing.T) {
	// Test require statement with single argument
	requireStmt := &RequireStmt{
		Args: []Expr{
			&BinaryExpr{
				Left:  &IdentExpr{Name: "amount"},
				Op:    ">",
				Right: &LiteralExpr{Value: "0"},
			},
		},
	}

	expected := "require!((amount > 0));"
	assert.Equal(t, expected, requireStmt.String())
}

func TestRequireStmtStringMultipleArgs(t *testing.T) {
	// Test require statement with multiple arguments
	requireStmt := &RequireStmt{
		Args: []Expr{
			&BinaryExpr{
				Left:  &IdentExpr{Name: "amount"},
				Op:    ">",
				Right: &LiteralExpr{Value: "0"},
			},
			&FieldAccessExpr{
				Target: &IdentExpr{Name: "errors"},
				Field:  "InvalidAmount",
			},
		},
	}

	expected := "require!((amount > 0), errors.InvalidAmount);"
	assert.Equal(t, expected, requireStmt.String())
}

func TestComplexContractString(t *testing.T) {
	// Create a complex contract with all new features
	contract := &Contract{
		Name: Ident{Value: "ERC20"},
		LeadingComments: []ContractItem{
			&Comment{Text: "// SPDX-License-Identifier: MIT"},
		},
		Items: []ContractItem{
			&Use{
				Namespaces: []*Namespace{
					{Name: Ident{Value: "std"}},
					{Name: Ident{Value: "evm"}},
				},
				Imports: []*ImportItem{
					{Name: Ident{Value: "sender"}},
				},
			},
			&Struct{
				Name:      Ident{Value: "State"},
				Attribute: &Attribute{Name: "storage"},
				Items: []StructItem{
					&StructField{
						Name: Ident{Value: "balance"},
						VariableType: &VariableType{
							Name: Ident{Value: "U256"},
						},
					},
				},
			},
			&Function{
				Name:      Ident{Value: "create"},
				External:  false,
				Attribute: &Attribute{Name: "create"},
				Params: []*FunctionParam{
					{
						Name: Ident{Value: "supply"},
						Type: &VariableType{
							Name: Ident{Value: "U256"},
						},
					},
				},
				Writes: []Ident{
					{Value: "State"},
				},
				Body: &FunctionBlock{
					Items: []FunctionBlockItem{
						&LetStmt{
							Name: Ident{Value: "total"},
							Expr: &IdentExpr{Name: "supply"},
							Mut:  true,
						},
						&RequireStmt{
							Args: []Expr{
								&BinaryExpr{
									Left:  &IdentExpr{Name: "total"},
									Op:    ">",
									Right: &LiteralExpr{Value: "0"},
								},
							},
						},
					},
				},
			},
		},
	}

	result := contract.String()

	// Verify structure
	assert.Contains(t, result, "// SPDX-License-Identifier: MIT")
	assert.Contains(t, result, "contract ERC20 {")
	assert.Contains(t, result, "use std::evm::{sender}")
	assert.Contains(t, result, "#[storage]")
	assert.Contains(t, result, "struct State")
	assert.Contains(t, result, "#[create]")
	assert.Contains(t, result, "fn create(supply: U256) writes(State) {")
	assert.Contains(t, result, "let mut total = supply;")
	assert.Contains(t, result, "require!((total > 0));")

	// Verify order: leading comments should come first
	licensePos := findSubstring(result, "// SPDX-License-Identifier")
	contractPos := findSubstring(result, "contract ERC20")
	assert.True(t, licensePos < contractPos, "License should appear before contract")
}

func TestFunctionStringWithReadsWrites(t *testing.T) {
	fn := &Function{
		Name:     Ident{Value: "transfer"},
		External: true,
		Params: []*FunctionParam{
			{Name: Ident{Value: "to"}, Type: &VariableType{Name: Ident{Value: "Address"}}},
			{Name: Ident{Value: "amount"}, Type: &VariableType{Name: Ident{Value: "U256"}}},
		},
		Return: &VariableType{Name: Ident{Value: "Bool"}},
		Reads:  []Ident{{Value: "Config"}},
		Writes: []Ident{{Value: "State"}},
		Body:   &FunctionBlock{},
	}

	result := fn.String()
	assert.Contains(t, result, "ext fn transfer(to: Address, amount: U256) -> Bool reads(Config) writes(State)")
}

// Helper function to find substring position
func findSubstring(text, substr string) int {
	for i := 0; i <= len(text)-len(substr); i++ {
		if text[i:i+len(substr)] == substr {
			return i
		}
	}
	return -1
}
