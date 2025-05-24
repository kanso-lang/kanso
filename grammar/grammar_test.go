package grammar_test

import (
	"github.com/stretchr/testify/assert"
	"kanso/grammar"
	"testing"
)

func TestERC20(t *testing.T) {
	program, err := grammar.ParseFile(`../examples/erc20.ka`)
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	assert.NotNil(t, program)
	assert.Equal(t, 2, len(program.SourceElements))

	comment := program.SourceElements[0]
	assert.NotNil(t, comment)
	assert.NotNil(t, comment.Comment)
	assert.Equal(t, "// SPDX-License-Identifier: Apache-2.0", comment.Comment.Text)

	module := program.SourceElements[1].Module
	assert.NotNil(t, module)
	assert.Equal(t, "ERC20", module.Name)
	assert.Equal(t, "contract", module.Attribute.Name)

	// Validate use statements
	assert.Equal(t, 4, len(module.Uses))
	assert.Equal(t, "Evm", module.Uses[0].Namespaces[0].Name)
	assert.Equal(t, "sender", module.Uses[0].Imports[0].Name)
	assert.Equal(t, "emit", module.Uses[0].Imports[1].Name)
	assert.Equal(t, "Table", module.Uses[1].Namespaces[0].Name)
	assert.Equal(t, "Self", module.Uses[1].Imports[0].Name)
	assert.Equal(t, "Table", module.Uses[1].Imports[1].Name)
	assert.Equal(t, "std", module.Uses[2].Namespaces[0].Name)
	assert.Equal(t, "ascii", module.Uses[2].Namespaces[1].Name)
	assert.Equal(t, "String", module.Uses[2].Imports[0].Name)
	assert.Equal(t, "std", module.Uses[3].Namespaces[0].Name)
	assert.Equal(t, "errors", module.Uses[3].Namespaces[1].Name)

	// Validate structs
	assert.Equal(t, 3, len(module.Structs))

	transferFields := map[string]string{
		"from":  "address",
		"to":    "address",
		"value": "u256",
	}
	checkStruct(t, module.Structs[0], "Transfer", "event", transferFields)

	approvalFields := map[string]string{
		"owner":   "address",
		"spender": "address",
		"value":   "u256",
	}
	checkStruct(t, module.Structs[1], "Approval", "event", approvalFields)

	stateFields := map[string]string{
		"balances":     "Table",
		"allowances":   "Table",
		"total_supply": "u256",
		"name":         "String",
		"symbol":       "String",
		"decimals":     "u8",
	}
	checkStruct(t, module.Structs[2], "State", "storage", stateFields)

	// Validate functions (just entry points; each has its own full checker)
	assert.Equal(t, 13, len(module.Functions))

	checkFunction(t, module.Functions[0], "create", "", map[string]string{
		"name":           "String",
		"symbol":         "String",
		"initial_supply": "u256",
		"decimals":       "u8",
	}, false, nil, []string{"State"})

	checkFunction(t, module.Functions[1], "name", "String", map[string]string{}, true, []string{"State"}, nil)

	checkFunction(t, module.Functions[2], "symbol", "String", map[string]string{}, true, []string{"State"}, nil)

	checkFunction(t, module.Functions[3], "decimals", "u8", map[string]string{}, true, nil, nil)

	checkFunction(t, module.Functions[4], "totalSupply", "u256", map[string]string{}, true, []string{"State"}, nil)

	checkFunction(t, module.Functions[5], "balanceOf", "u256", map[string]string{
		"owner": "address",
	}, true, []string{"State"}, nil)

	checkFunction(t, module.Functions[6], "transfer", "bool", map[string]string{
		"to":     "address",
		"amount": "u256",
	}, true, nil, []string{"State"})

	checkFunction(t, module.Functions[7], "transferFrom", "bool", map[string]string{
		"from":   "address",
		"to":     "address",
		"amount": "u256",
	}, true, nil, []string{"State"})

	checkFunction(t, module.Functions[8], "approve", "bool", map[string]string{
		"spender": "address",
		"amount":  "u256",
	}, true, nil, []string{"State"})

	checkFunction(t, module.Functions[9], "allowance", "u256", map[string]string{
		"owner":   "address",
		"spender": "address",
	}, true, []string{"State"}, nil)

	checkFunction(t, module.Functions[10], "do_transfer", "", map[string]string{
		"from":   "address",
		"to":     "address",
		"amount": "u256",
	}, false, nil, []string{"State"})

	checkFunction(t, module.Functions[11], "mut_balanceOf", "&mut u256", map[string]string{
		"s":     "&mut State",
		"owner": "address",
	}, false, nil, nil)

	checkFunction(t, module.Functions[12], "mint", "", map[string]string{
		"account": "address",
		"amount":  "u256",
	}, false, nil, []string{"State"})

	//checkFunction_Create(t, module.Functions[0])
	//checkFunction_Name(t, module.Functions[1])
	//checkFunction_Symbol(t, module.Functions[2])
	//checkFunction_Decimals(t, module.Functions[3])
	//checkFunction_TotalSupply(t, module.Functions[4])
	//checkFunction_BalanceOf(t, module.Functions[5])
	//checkFunction_Transfer(t, module.Functions[6])
	//checkFunction_TransferFrom(t, module.Functions[7])
	//checkFunction_Approve(t, module.Functions[8])
	//checkFunction_Allowance(t, module.Functions[9])
	//checkFunction_DoTransfer(t, module.Functions[10])
	//checkFunction_MutBalanceOf(t, module.Functions[11])
	//checkFunction_Mint(t, module.Functions[12])
}

func checkStruct(t *testing.T, s *grammar.Struct, name string, attribute string, fields map[string]string) {
	assert.Equal(t, name, s.Name)

	if attribute != "" {
		assert.Equal(t, attribute, s.Attribute.Name)
	}

	assert.Equal(t, len(fields), len(s.Fields))

	i := 0
	for _, f := range s.Fields {
		value, exists := fields[f.Name]
		assert.True(t, exists)

		assert.Equal(t, value, f.Type.Name)
		i++
	}
}

func checkFunction(t *testing.T, f *grammar.Function, name string, returnType string, params map[string]string, public bool, reads []string, writes []string) {
	assert.Equal(t, name, f.Name)
	assert.Equal(t, public, f.Public)

	if returnType != "" {
		assert.NotNil(t, f.Return)

		if f.Return.Ref != nil {
			// This is a reference type (& or &mut)
			refPrefix := "&"
			if f.Return.Ref.Mut {
				refPrefix = "&mut"
			}
			actualType := f.Return.Ref.Target.Name
			assert.Equal(t, returnType, refPrefix+" "+actualType)
		} else {
			// Simple type
			assert.Equal(t, returnType, f.Return.Name)
		}
	} else {
		assert.Nil(t, f.Return)
	}

	assert.Equal(t, len(params), len(f.Params))
	for _, p := range f.Params {
		expectedType, exists := params[p.Name]
		assert.True(t, exists, "expected param %q to exist", p.Name)

		var actualType string
		if p.Type.Ref != nil {
			refPrefix := "&"
			if p.Type.Ref.Mut {
				refPrefix = "&mut"
			}
			actualType = refPrefix + " " + p.Type.Ref.Target.Name
		} else {
			actualType = p.Type.Name
		}

		assert.Equal(t, expectedType, actualType, "param %q type mismatch", p.Name)
	}

	if reads != nil {
		assert.Equal(t, len(reads), len(f.Reads))
		for i, r := range f.Reads {
			assert.Equal(t, reads[i], r.Name)
		}
	}
	if writes != nil {
		assert.Equal(t, len(writes), len(f.Writes))
		for i, w := range f.Writes {
			assert.Equal(t, writes[i], w.Name)
		}
	}
}
