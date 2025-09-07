package types

import (
	"kanso/internal/ast"
	"kanso/internal/stdlib"
	"strings"
)

// ImportParser handles parsing and validating use statements
type ImportParser struct {
	typeRegistry     *TypeRegistry
	functionRegistry FunctionRegistryInterface
	moduleRegistry   ModuleRegistryInterface
}

// FunctionRegistryInterface defines the interface for function registries
type FunctionRegistryInterface interface {
	AddImportedFunction(name, modulePath string)
}

// ModuleRegistryInterface defines the interface for module registries
type ModuleRegistryInterface interface {
	AddImportedModule(name, modulePath string)
}

// NewImportParser creates a new import parser
func NewImportParser(typeRegistry *TypeRegistry, functionRegistry FunctionRegistryInterface, moduleRegistry ModuleRegistryInterface) *ImportParser {
	return &ImportParser{
		typeRegistry:     typeRegistry,
		functionRegistry: functionRegistry,
		moduleRegistry:   moduleRegistry,
	}
}

// ParseUseStatement processes a use statement and updates the type registry
func (ip *ImportParser) ParseUseStatement(useStmt *ast.Use) []string {
	var errors []string

	// Build module path from namespaces
	var modulePath strings.Builder
	for i, ns := range useStmt.Namespaces {
		if i > 0 {
			modulePath.WriteString("::")
		}
		modulePath.WriteString(ns.Name.Value)
	}
	modulePathStr := modulePath.String()

	// Handle different use statement patterns
	for _, item := range useStmt.Imports {
		switch item.Name.Value {
		case "Self":
			// use Table::{Self, Table} - imports the module itself for static function calls
			// Add to module registry for calls like Table::empty()
			ip.moduleRegistry.AddImportedModule(getModuleName(modulePathStr), modulePathStr)

		default:
			// Check if this is a function import (not a type)
			if ip.isKnownFunction(item.Name.Value, modulePathStr) {
				// Add to function registry for calls like sender()
				ip.functionRegistry.AddImportedFunction(item.Name.Value, modulePathStr)
				continue
			}

			// Regular type import - determine if it's generic
			isGeneric := ip.isKnownGenericType(item.Name.Value, modulePathStr)
			ip.typeRegistry.AddImportedType(item.Name.Value, modulePathStr, isGeneric)
		}
	}

	// Handle module-level imports (without braces)
	if len(useStmt.Imports) == 0 {
		// use std::errors - imports the entire module
		ip.moduleRegistry.AddImportedModule(getModuleName(modulePathStr), modulePathStr)
	}

	return errors
}

// isKnownGenericType checks if a type from a specific module is known to be generic
func (ip *ImportParser) isKnownGenericType(typeName, modulePath string) bool {
	// Use standard library definitions
	if moduleDef := stdlib.GetModuleDefinition(modulePath); moduleDef != nil {
		if typeDef, exists := moduleDef.Types[typeName]; exists {
			return typeDef.IsGeneric
		}
	}

	return false
}

// isKnownFunction checks if an import is a known function (not a type)
func (ip *ImportParser) isKnownFunction(functionName, modulePath string) bool {
	// Use standard library definitions
	if moduleDef := stdlib.GetModuleDefinition(modulePath); moduleDef != nil {
		_, exists := moduleDef.Functions[functionName]
		return exists
	}

	return false
}

// getModuleName extracts the module name from a module path
func getModuleName(modulePath string) string {
	parts := strings.Split(modulePath, "::")
	return parts[len(parts)-1]
}

// ValidateImportedTypeUsage validates that imported generic types are used correctly
func (ip *ImportParser) ValidateImportedTypeUsage(typeName string, hasGenerics bool) []string {
	var errors []string

	importedType := ip.typeRegistry.GetImportedType(typeName)
	if importedType == nil {
		return errors // Not an imported type
	}

	// Check generic usage
	if importedType.IsGeneric && !hasGenerics {
		errors = append(errors, "generic type "+typeName+" requires type parameters")
	} else if !importedType.IsGeneric && hasGenerics {
		errors = append(errors, "type "+typeName+" does not accept type parameters")
	}

	return errors
}
