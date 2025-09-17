package ir

// This file provides the main entry point for the IR system
// The IR is implemented using Static Single Assignment (SSA) form for optimal EVM optimization

import (
	"kanso/internal/ast"
	"kanso/internal/semantic"
)

// BuildProgram is the main entry point for converting AST to IR
func BuildProgram(contract *ast.Contract, context *semantic.ContextRegistry) *Program {
	builder := NewBuilder(context)
	program := builder.Build(contract)

	// Temporarily disable optimization pipeline to see full IR
	// pipeline := NewOptimizationPipeline()
	// pipeline.Run(program)

	return program
}

// PrintProgram returns a pretty-printed representation of the IR
func PrintProgram(program *Program) string {
	return Print(program)
}
