package semantic

import (
	"kanso/internal/ast"
	"kanso/internal/stdlib"
	"kanso/internal/types"
)

// ContextRegistry provides a unified view of all available types, functions, and modules
// in a specific semantic analysis context
type ContextRegistry struct {
	// Core registries
	typeRegistry     *types.TypeRegistry
	functionRegistry *FunctionRegistry
	moduleRegistry   *ModuleRegistry

	// Standard library modules for reference
	stdlibModules map[string]*stdlib.ModuleDefinition
}

// NewContextRegistry creates a new unified context registry
func NewContextRegistry() *ContextRegistry {
	typeRegistry := types.NewTypeRegistry()
	typeRegistry.InitializeBuiltins()

	return &ContextRegistry{
		typeRegistry:     typeRegistry,
		functionRegistry: NewFunctionRegistry(),
		moduleRegistry:   NewModuleRegistry(),
		stdlibModules:    stdlib.GetStandardModules(),
	}
}

// Types - delegate to TypeRegistry

// IsValidType checks if a type is valid in this context
func (cr *ContextRegistry) IsValidType(typeName string) bool {
	return cr.typeRegistry.IsValidType(typeName)
}

// IsBuiltinType checks if a type is a built-in type
func (cr *ContextRegistry) IsBuiltinType(typeName string) bool {
	return cr.typeRegistry.IsBuiltinType(typeName)
}

// IsImportedType checks if a type is imported
func (cr *ContextRegistry) IsImportedType(typeName string) bool {
	return cr.typeRegistry.IsImportedType(typeName)
}

// IsUserDefinedType checks if a type is user-defined
func (cr *ContextRegistry) IsUserDefinedType(typeName string) bool {
	return cr.typeRegistry.IsUserDefinedType(typeName)
}

// AddUserDefinedType adds a user-defined struct to the registry
func (cr *ContextRegistry) AddUserDefinedType(name string, structDef *ast.Struct) {
	cr.typeRegistry.AddUserDefinedType(name, structDef)
}

// GetUserDefinedType returns a user-defined struct definition
func (cr *ContextRegistry) GetUserDefinedType(typeName string) *ast.Struct {
	return cr.typeRegistry.GetUserDefinedType(typeName)
}

// GetImportedType returns information about an imported type
func (cr *ContextRegistry) GetImportedType(typeName string) *types.ImportedType {
	return cr.typeRegistry.GetImportedType(typeName)
}

// Functions - delegate to FunctionRegistry

// IsImportedFunction checks if a function is imported
func (cr *ContextRegistry) IsImportedFunction(functionName string) bool {
	return cr.functionRegistry.IsImportedFunction(functionName)
}

// GetImportedFunction returns information about an imported function
func (cr *ContextRegistry) GetImportedFunction(functionName string) *ImportedFunction {
	return cr.functionRegistry.GetImportedFunction(functionName)
}

// Modules - delegate to ModuleRegistry

// IsImportedModule checks if a module is imported
func (cr *ContextRegistry) IsImportedModule(moduleName string) bool {
	return cr.moduleRegistry.IsImportedModule(moduleName)
}

// GetImportedModule returns information about an imported module
func (cr *ContextRegistry) GetImportedModule(moduleName string) *ImportedModule {
	return cr.moduleRegistry.GetImportedModule(moduleName)
}

// Import handling - unified interface for processing imports

// ProcessUseStatement processes a use statement and updates all relevant registries
func (cr *ContextRegistry) ProcessUseStatement(useStmt *ast.Use) []string {
	// Use the existing ImportParser but with our registries
	importParser := types.NewImportParser(cr.typeRegistry, cr.functionRegistry, cr.moduleRegistry)
	return importParser.ParseUseStatement(useStmt)
}

// Standard library queries

// IsStandardModule checks if a module path is a known standard library module
func (cr *ContextRegistry) IsStandardModule(modulePath string) bool {
	_, exists := cr.stdlibModules[modulePath]
	return exists
}

// GetStandardModuleDefinition returns the definition for a standard library module
func (cr *ContextRegistry) GetStandardModuleDefinition(modulePath string) *stdlib.ModuleDefinition {
	return cr.stdlibModules[modulePath]
}

// GetFunctionDefinition returns the complete function definition for an imported function
func (cr *ContextRegistry) GetFunctionDefinition(functionName string) *stdlib.FunctionDefinition {
	if importedFunc := cr.GetImportedFunction(functionName); importedFunc != nil {
		if moduleDef := cr.GetStandardModuleDefinition(importedFunc.ModulePath); moduleDef != nil {
			if funcDef, exists := moduleDef.Functions[functionName]; exists {
				return &funcDef
			}
		}
	}
	return nil
}

// GetModuleFunctionDefinition returns a function definition from a module
func (cr *ContextRegistry) GetModuleFunctionDefinition(moduleName, functionName string) *stdlib.FunctionDefinition {
	if importedModule := cr.GetImportedModule(moduleName); importedModule != nil {
		if moduleDef := cr.GetStandardModuleDefinition(importedModule.ModulePath); moduleDef != nil {
			if funcDef, exists := moduleDef.Functions[functionName]; exists {
				return &funcDef
			}
		}
	}
	return nil
}

// Context validation

// ValidateTypeUsage validates that a type is used correctly in context
func (cr *ContextRegistry) ValidateTypeUsage(typeName string, hasGenerics bool) []string {
	var errors []string

	// Check if it's an imported type that needs validation
	if importedType := cr.GetImportedType(typeName); importedType != nil {
		if importedType.IsGeneric && !hasGenerics {
			errors = append(errors, "generic type "+typeName+" requires type parameters")
		} else if !importedType.IsGeneric && hasGenerics {
			errors = append(errors, "type "+typeName+" does not accept type parameters")
		}
	}

	return errors
}

// ValidateFunctionCall validates that a function call is valid in context
func (cr *ContextRegistry) ValidateFunctionCall(functionName string) []string {
	var errors []string

	// Check if function is available (either imported or from imported module)
	if !cr.IsImportedFunction(functionName) {
		// Could also check if it's available via module access (e.g., Module::function)
		// For now, just check direct imports
		errors = append(errors, "function "+functionName+" is not imported or defined")
	}

	return errors
}

// ValidateModuleAccess validates that a module access is valid (e.g., Module::function)
func (cr *ContextRegistry) ValidateModuleAccess(moduleName, itemName string) []string {
	var errors []string

	// Check if module is imported
	if !cr.IsImportedModule(moduleName) {
		errors = append(errors, "module "+moduleName+" is not imported")
		return errors
	}

	// Check if the accessed item exists in the standard library definition
	if stdModule := cr.GetStandardModuleDefinition(moduleName); stdModule != nil {
		// This would need more sophisticated checking based on what itemName represents
		// (function, type, etc.)
	}

	return errors
}
