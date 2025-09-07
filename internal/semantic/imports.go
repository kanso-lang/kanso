package semantic

// ImportedFunction represents a function imported via use statement
type ImportedFunction struct {
	Name       string // The function name (e.g., "sender", "emit")
	ModulePath string // The module it's imported from (e.g., "Evm")
}

// ImportedModule represents a module imported via use statement
type ImportedModule struct {
	Name       string // The module name (e.g., "errors", "Table")
	ModulePath string // The full module path (e.g., "std::errors", "Table")
}

// FunctionRegistry manages available imported functions
type FunctionRegistry struct {
	functions map[string]*ImportedFunction
}

// ModuleRegistry manages available imported modules
type ModuleRegistry struct {
	modules map[string]*ImportedModule
}

// NewFunctionRegistry creates a new function registry
func NewFunctionRegistry() *FunctionRegistry {
	return &FunctionRegistry{
		functions: make(map[string]*ImportedFunction),
	}
}

// NewModuleRegistry creates a new module registry
func NewModuleRegistry() *ModuleRegistry {
	return &ModuleRegistry{
		modules: make(map[string]*ImportedModule),
	}
}

// FunctionRegistry methods

// AddImportedFunction adds an imported function to the registry
func (fr *FunctionRegistry) AddImportedFunction(name, modulePath string) {
	fr.functions[name] = &ImportedFunction{
		Name:       name,
		ModulePath: modulePath,
	}
}

// IsImportedFunction checks if a function is imported
func (fr *FunctionRegistry) IsImportedFunction(functionName string) bool {
	return fr.functions[functionName] != nil
}

// GetImportedFunction returns information about an imported function
func (fr *FunctionRegistry) GetImportedFunction(functionName string) *ImportedFunction {
	return fr.functions[functionName]
}

// ModuleRegistry methods

// AddImportedModule adds an imported module to the registry
func (mr *ModuleRegistry) AddImportedModule(name, modulePath string) {
	mr.modules[name] = &ImportedModule{
		Name:       name,
		ModulePath: modulePath,
	}
}

// IsImportedModule checks if a module is imported
func (mr *ModuleRegistry) IsImportedModule(moduleName string) bool {
	return mr.modules[moduleName] != nil
}

// GetImportedModule returns information about an imported module
func (mr *ModuleRegistry) GetImportedModule(moduleName string) *ImportedModule {
	return mr.modules[moduleName]
}
