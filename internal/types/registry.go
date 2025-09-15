package types

import "kanso/internal/ast"

// ImportedType represents a type imported via use statement
type ImportedType struct {
	Name       string // The type name (e.g., "Slots", "String")
	ModulePath string // The module it's imported from (e.g., "std::evm", "std::ascii")
	IsGeneric  bool   // Whether the type accepts generic parameters
}

// TypeRegistry manages available types in a specific scope
type TypeRegistry struct {
	builtins    map[string]bool
	imports     map[string]*ImportedType
	userDefined map[string]*ast.Struct // Structs defined in current module
}

// NewTypeRegistry creates a new type registry with built-in types
func NewTypeRegistry() *TypeRegistry {
	return &TypeRegistry{
		builtins:    make(map[string]bool),
		imports:     make(map[string]*ImportedType),
		userDefined: make(map[string]*ast.Struct),
	}
}

// InitializeBuiltins adds all built-in types to the registry
func (tr *TypeRegistry) InitializeBuiltins() {
	for typeName := range BuiltinTypes {
		tr.builtins[typeName] = true
	}
}

// AddImportedType adds an imported type to the registry
func (tr *TypeRegistry) AddImportedType(name, modulePath string, isGeneric bool) {
	tr.imports[name] = &ImportedType{
		Name:       name,
		ModulePath: modulePath,
		IsGeneric:  isGeneric,
	}
}

// AddUserDefinedType adds a user-defined struct to the registry
func (tr *TypeRegistry) AddUserDefinedType(name string, structDef *ast.Struct) {
	tr.userDefined[name] = structDef
}

// IsValidType checks if a type name is valid in this registry
func (tr *TypeRegistry) IsValidType(typeName string) bool {
	// Check built-ins
	if tr.builtins[typeName] {
		return true
	}

	// Check imports
	if tr.imports[typeName] != nil {
		return true
	}

	// Check user-defined types
	if tr.userDefined[typeName] != nil {
		return true
	}

	return false
}

// IsBuiltinType checks if a type is a built-in type
func (tr *TypeRegistry) IsBuiltinType(typeName string) bool {
	return tr.builtins[typeName]
}

// IsImportedType checks if a type is imported
func (tr *TypeRegistry) IsImportedType(typeName string) bool {
	return tr.imports[typeName] != nil
}

// IsUserDefinedType checks if a type is user-defined (struct)
func (tr *TypeRegistry) IsUserDefinedType(typeName string) bool {
	return tr.userDefined[typeName] != nil
}

// GetImportedType returns information about an imported type
func (tr *TypeRegistry) GetImportedType(typeName string) *ImportedType {
	return tr.imports[typeName]
}

// GetUserDefinedType returns the struct definition for a user-defined type
func (tr *TypeRegistry) GetUserDefinedType(typeName string) *ast.Struct {
	return tr.userDefined[typeName]
}
