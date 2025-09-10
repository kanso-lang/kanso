package stdlib

import "kanso/internal/builtins"

// ModuleDefinition defines a standard library module
type ModuleDefinition struct {
	Name      string                        // Module name (e.g., "Evm", "Table")
	Path      string                        // Full module path (e.g., "Evm", "std::ascii")
	Types     map[string]TypeDefinition     // Available types in this module
	Functions map[string]FunctionDefinition // Available functions in this module
}

// TypeDefinition defines a type from a standard library module
type TypeDefinition struct {
	Name      string // Type name (e.g., "Table", "String")
	IsGeneric bool   // Whether the type accepts generic parameters
}

// FunctionDefinition defines a function signature from a standard library module
type FunctionDefinition struct {
	Name       string                // Function name (e.g., "empty", "sender")
	Parameters []ParameterDefinition // Function parameters
	ReturnType *TypeRef              // Return type (nil if void)
	IsGeneric  bool                  // Whether the function has generic type parameters
}

// ParameterDefinition defines a function parameter
type ParameterDefinition struct {
	Name string   // Parameter name
	Type *TypeRef // Parameter type
}

// TypeRef represents a type reference that can be generic
type TypeRef struct {
	Name        string     // Base type name (e.g., "Table", "u32", "T")
	IsGeneric   bool       // Whether this is a generic type parameter (T, K, V)
	GenericArgs []*TypeRef // Generic type arguments for parameterized types
}

// Helper functions for creating type references
func NewTypeRef(name string) *TypeRef {
	return &TypeRef{Name: name, IsGeneric: false}
}

// Built-in type references using actual builtins.BuiltinType constants
func AddressType() *TypeRef {
	return &TypeRef{Name: string(builtins.Address), IsGeneric: false}
}

func BoolType() *TypeRef {
	return &TypeRef{Name: string(builtins.Bool), IsGeneric: false}
}

func U8Type() *TypeRef {
	return &TypeRef{Name: string(builtins.U8), IsGeneric: false}
}

func U16Type() *TypeRef {
	return &TypeRef{Name: string(builtins.U16), IsGeneric: false}
}

func U32Type() *TypeRef {
	return &TypeRef{Name: string(builtins.U32), IsGeneric: false}
}

func U64Type() *TypeRef {
	return &TypeRef{Name: string(builtins.U64), IsGeneric: false}
}

func U128Type() *TypeRef {
	return &TypeRef{Name: string(builtins.U128), IsGeneric: false}
}

func U256Type() *TypeRef {
	return &TypeRef{Name: string(builtins.U256), IsGeneric: false}
}

func NewGenericTypeRef(name string, args ...*TypeRef) *TypeRef {
	return &TypeRef{Name: name, IsGeneric: false, GenericArgs: args}
}

func NewGenericParam(name string) *TypeRef {
	return &TypeRef{Name: name, IsGeneric: true}
}

// Helper function for creating function definitions
func NewFunction(name string, returnType *TypeRef, params ...ParameterDefinition) FunctionDefinition {
	return FunctionDefinition{
		Name:       name,
		Parameters: params,
		ReturnType: returnType,
		IsGeneric:  false,
	}
}

func NewGenericFunction(name string, returnType *TypeRef, params ...ParameterDefinition) FunctionDefinition {
	return FunctionDefinition{
		Name:       name,
		Parameters: params,
		ReturnType: returnType,
		IsGeneric:  true,
	}
}

// Helper function for creating parameters
func NewParam(name string, typeRef *TypeRef) ParameterDefinition {
	return ParameterDefinition{Name: name, Type: typeRef}
}

// GetStandardModules returns all built-in standard library modules
func GetStandardModules() map[string]*ModuleDefinition {
	// Create shared evm module definition for backward compatibility
	evmModule := &ModuleDefinition{
		Name:  "Evm",
		Path:  "Evm",
		Types: map[string]TypeDefinition{
			// Evm module doesn't export types, only functions
		},
		Functions: map[string]FunctionDefinition{
			"sender": NewFunction("sender", AddressType()),
			"emit":   NewFunction("emit", nil, NewParam("event", NewGenericParam("T"))),
		},
	}

	// Create std::evm module definition
	stdEvmModule := &ModuleDefinition{
		Name:  "evm",
		Path:  "std::evm",
		Types: map[string]TypeDefinition{
			// Evm module doesn't export types, only functions
		},
		Functions: map[string]FunctionDefinition{
			"sender": NewFunction("sender", AddressType()),
			"emit":   NewFunction("emit", nil, NewParam("event", NewGenericParam("T"))),
		},
	}

	return map[string]*ModuleDefinition{
		"Evm":      evmModule,    // Backward compatibility
		"std::evm": stdEvmModule, // New style
		"Table": {
			Name: "Table",
			Path: "Table",
			Types: map[string]TypeDefinition{
				"Table": {Name: "Table", IsGeneric: true},
			},
			Functions: map[string]FunctionDefinition{
				"empty": NewGenericFunction("empty",
					NewGenericTypeRef("Table", NewGenericParam("K"), NewGenericParam("V"))),
				"borrow": NewGenericFunction("borrow",
					NewGenericParam("V"),
					NewParam("table", NewGenericTypeRef("Table", NewGenericParam("K"), NewGenericParam("V"))),
					NewParam("key", NewGenericParam("K"))),
				"borrow_with_default": NewGenericFunction("borrow_with_default",
					NewGenericParam("V"),
					NewParam("table", NewGenericTypeRef("Table", NewGenericParam("K"), NewGenericParam("V"))),
					NewParam("key", NewGenericParam("K")),
					NewParam("default", NewGenericParam("V"))),
				"borrow_mut": NewGenericFunction("borrow_mut",
					NewGenericParam("V"),
					NewParam("table", NewGenericTypeRef("Table", NewGenericParam("K"), NewGenericParam("V"))),
					NewParam("key", NewGenericParam("K"))),
				"borrow_mut_with_default": NewGenericFunction("borrow_mut_with_default",
					NewGenericParam("V"),
					NewParam("table", NewGenericTypeRef("Table", NewGenericParam("K"), NewGenericParam("V"))),
					NewParam("key", NewGenericParam("K")),
					NewParam("default", NewGenericParam("V"))),
				"insert": NewGenericFunction("insert",
					nil,
					NewParam("table", NewGenericTypeRef("Table", NewGenericParam("K"), NewGenericParam("V"))),
					NewParam("key", NewGenericParam("K")),
					NewParam("value", NewGenericParam("V"))),
				"delete": NewGenericFunction("delete",
					NewGenericParam("V"),
					NewParam("table", NewGenericTypeRef("Table", NewGenericParam("K"), NewGenericParam("V"))),
					NewParam("key", NewGenericParam("K"))),
			},
		},
		"std::address": {
			Name:  "address",
			Path:  "std::address",
			Types: map[string]TypeDefinition{
				// address module doesn't export types, only functions
			},
			Functions: map[string]FunctionDefinition{
				"zero": NewFunction("zero", AddressType()),
			},
		},
		"std::ascii": {
			Name: "ascii",
			Path: "std::ascii",
			Types: map[string]TypeDefinition{
				"String": {Name: "String", IsGeneric: false},
			},
			Functions: map[string]FunctionDefinition{
				// String manipulation functions would go here - empty for now
			},
		},
		"std::errors": {
			Name:  "errors",
			Path:  "std::errors",
			Types: map[string]TypeDefinition{
				// errors module doesn't export types, only functions
			},
			Functions: map[string]FunctionDefinition{
				"invalid_argument":      NewFunction("invalid_argument", U64Type(), NewParam("code", U64Type())),
				"limit_exceeded":        NewFunction("limit_exceeded", U64Type(), NewParam("code", U64Type())),
				"SelfTransfer":          NewFunction("SelfTransfer", U64Type()),
				"InsufficientAllowance": NewFunction("InsufficientAllowance", U64Type()),
				"InsufficientBalance":   NewFunction("InsufficientBalance", U64Type()),
			},
		},
		"std::vector": {
			Name: "vector",
			Path: "std::vector",
			Types: map[string]TypeDefinition{
				"vector": {Name: "vector", IsGeneric: true},
			},
			Functions: map[string]FunctionDefinition{
				"empty": NewGenericFunction("empty", NewGenericTypeRef("vector", NewGenericParam("T"))),
				"push_back": NewGenericFunction("push_back", nil,
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T"))),
					NewParam("item", NewGenericParam("T"))),
				"pop_back": NewGenericFunction("pop_back", NewGenericParam("T"),
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T")))),
				"borrow": NewGenericFunction("borrow", NewGenericParam("T"),
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T"))),
					NewParam("index", U64Type())),
				"borrow_mut": NewGenericFunction("borrow_mut", NewGenericParam("T"),
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T"))),
					NewParam("index", U64Type())),
				"append": NewGenericFunction("append", nil,
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T"))),
					NewParam("other", NewGenericTypeRef("vector", NewGenericParam("T")))),
				"contains": NewGenericFunction("contains", BoolType(),
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T"))),
					NewParam("item", NewGenericParam("T"))),
				"swap": NewGenericFunction("swap", nil,
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T"))),
					NewParam("i", U64Type()),
					NewParam("j", U64Type())),
				"reverse": NewGenericFunction("reverse", nil,
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T")))),
				"index_of": NewGenericFunction("index_of", U64Type(),
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T"))),
					NewParam("item", NewGenericParam("T"))),
				"remove": NewGenericFunction("remove", NewGenericParam("T"),
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T"))),
					NewParam("index", U64Type())),
				"swap_remove": NewGenericFunction("swap_remove", NewGenericParam("T"),
					NewParam("vec", NewGenericTypeRef("vector", NewGenericParam("T"))),
					NewParam("index", U64Type())),
			},
		},
	}
}

// IsKnownModule checks if a module path is a known standard library module
func IsKnownModule(modulePath string) bool {
	modules := GetStandardModules()
	_, exists := modules[modulePath]
	return exists
}

// GetModuleDefinition returns the definition for a standard library module
func GetModuleDefinition(modulePath string) *ModuleDefinition {
	modules := GetStandardModules()
	return modules[modulePath]
}
