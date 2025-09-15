package stdlib

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestGetStandardModules(t *testing.T) {
	modules := GetStandardModules()

	// Verify core modules exist
	assert.NotNil(t, modules["std::ascii"], "std::ascii module should exist")
	assert.NotNil(t, modules["std::errors"], "std::errors module should exist")

	// Verify Evm module details
	evm := modules["std::evm"]
	assert.Equal(t, "evm", evm.Name)
	assert.Equal(t, "std::evm", evm.Path)

	_, hasSender := evm.Functions["sender"]
	assert.True(t, hasSender, "std::evm should have sender function")

	_, hasEmit := evm.Functions["emit"]
	assert.True(t, hasEmit, "std::evm should have emit function")
	assert.Empty(t, evm.Types, "std::evm should not export types")

	// Verify function signatures
	senderFunc := evm.Functions["sender"]
	assert.Equal(t, "sender", senderFunc.Name)
	assert.Equal(t, "Address", senderFunc.ReturnType.Name)
	assert.Empty(t, senderFunc.Parameters)

	emitFunc := evm.Functions["emit"]
	assert.Equal(t, "emit", emitFunc.Name)
	assert.Nil(t, emitFunc.ReturnType) // void function
	assert.Len(t, emitFunc.Parameters, 1)
	assert.Equal(t, "event", emitFunc.Parameters[0].Name)

	// Verify std::ascii module details
	ascii := modules["std::ascii"]
	assert.Equal(t, "ascii", ascii.Name)
	assert.Equal(t, "std::ascii", ascii.Path)
	assert.False(t, ascii.Types["String"].IsGeneric, "String type should not be generic")
}

func TestIsKnownModule(t *testing.T) {
	assert.True(t, IsKnownModule("std::evm"), "std::evm should be known")
	assert.True(t, IsKnownModule("std::ascii"), "std::ascii should be known")
	assert.True(t, IsKnownModule("std::errors"), "std::errors should be known")
	assert.False(t, IsKnownModule("UnknownModule"), "UnknownModule should not be known")
}

func TestGetModuleDefinition(t *testing.T) {
	// Test existing module
	evm := GetModuleDefinition("std::evm")
	assert.NotNil(t, evm, "Should return std::evm module definition")
	assert.Equal(t, "evm", evm.Name)

	// Test non-existing module
	unknown := GetModuleDefinition("UnknownModule")
	assert.Nil(t, unknown, "Should return nil for unknown module")
}
