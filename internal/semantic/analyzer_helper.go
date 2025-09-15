package semantic

import (
	"kanso/internal/ast"
	"kanso/internal/stdlib"
	"math/big"
)

func (a *Analyzer) findSimilarVariables(name string) []string {
	var similar []string

	// Check current scope and parent scopes
	for scope := a.symbols; scope != nil; scope = scope.parent {
		for varName := range scope.symbols {
			if levenshteinDistance(name, varName) <= 2 && len(varName) > 1 {
				similar = append(similar, varName)
			}
		}
	}

	return similar
}

func (a *Analyzer) findSimilarFunctions(name string) []string {
	var similar []string

	// Check local functions
	for funcName := range a.localFunctions {
		if levenshteinDistance(name, funcName) <= 2 && len(funcName) > 1 {
			similar = append(similar, funcName)
		}
	}

	// Check imported functions
	importedFunctions := a.context.GetAllImportedFunctions()
	for _, funcName := range importedFunctions {
		if levenshteinDistance(name, funcName) <= 2 && len(funcName) > 1 {
			similar = append(similar, funcName)
		}
	}

	return similar
}

func (a *Analyzer) findPossibleImports(name string) []string {
	return a.findSmartImportSuggestions(name)
}

func (a *Analyzer) findSmartImportSuggestions(name string) []string {
	// Find all possible functions that could be imported
	possibleImports := make(map[string][]string) // modulePath -> []functionNames

	modules := a.context.GetStandardModules()
	for modulePath, module := range modules {
		for funcName, _ := range module.Functions {
			if levenshteinDistance(name, funcName) <= 2 && len(funcName) > 1 {
				// Skip if this function is already imported
				if !a.context.IsImportedFunction(funcName) {
					possibleImports[modulePath] = append(possibleImports[modulePath], funcName)
				}
			}
		}
	}

	// Group by module and check existing imports
	result := make([]string, 0)

	for modulePath, newFunctions := range possibleImports {
		// Check if we already have an import for this module
		existingImport := a.findExistingImportFor(modulePath)

		if existingImport != nil {
			// We already import from this module, suggest extending the import
			combinedImport := a.buildCombinedImport(existingImport, newFunctions)
			if combinedImport != "" {
				// Only suggest the extended import, not standalone imports for this module
				result = append(result, combinedImport)
			}
		} else {
			// No existing import, suggest new import
			for _, funcName := range newFunctions {
				result = append(result, modulePath+"::{"+funcName+"}")
			}
		}
	}

	return result
}

func (a *Analyzer) findExistingImportFor(modulePath string) *ast.Use {
	for _, useStmt := range a.existingUseStmts {
		// Build the module path from namespaces
		currentPath := a.buildModulePath(useStmt.Namespaces)
		if currentPath == modulePath {
			return useStmt
		}
	}
	return nil
}

func (a *Analyzer) buildModulePath(namespaces []*ast.Namespace) string {
	if len(namespaces) == 0 {
		return ""
	}

	path := ""
	for i, ns := range namespaces {
		if i > 0 {
			path += "::"
		}
		path += ns.Name.Value
	}
	return path
}

func (a *Analyzer) buildCombinedImport(existingImport *ast.Use, newFunctions []string) string {
	// Extract existing imported items
	existingItems := make([]string, 0)

	if existingImport.Imports != nil {
		for _, item := range existingImport.Imports {
			existingItems = append(existingItems, item.Name.Value)
		}
	}

	// Combine existing and new items, removing duplicates and invalid items
	allItems := make(map[string]bool)

	// Only include existing items that are valid functions
	modulePath := a.buildModulePath(existingImport.Namespaces)
	modules := a.context.GetStandardModules()
	if moduleDef, exists := modules[modulePath]; exists {
		for _, item := range existingItems {
			// Only include existing items that are actually valid functions or types
			_, isFunction := moduleDef.Functions[item]
			_, isType := moduleDef.Types[item]
			if isFunction || isType {
				allItems[item] = true
			}
		}
	}

	for _, newFunc := range newFunctions {
		allItems[newFunc] = true
	}

	// Convert to sorted slice for consistent output
	combinedItems := make([]string, 0, len(allItems))
	for item := range allItems {
		combinedItems = append(combinedItems, item)
	}

	// Sort alphabetically
	for i := 0; i < len(combinedItems); i++ {
		for j := i + 1; j < len(combinedItems); j++ {
			if combinedItems[i] > combinedItems[j] {
				combinedItems[i], combinedItems[j] = combinedItems[j], combinedItems[i]
			}
		}
	}

	// Build the import suggestion
	if len(combinedItems) == 0 {
		return ""
	}

	itemsStr := ""
	for i, item := range combinedItems {
		if i > 0 {
			itemsStr += ", "
		}
		itemsStr += item
	}

	return a.buildModulePath(existingImport.Namespaces) + "::{" + itemsStr + "}"
}

func (a *Analyzer) getStructFields(structName string) []string {
	var fields []string

	structDef := a.context.GetUserDefinedType(structName)
	if structDef != nil {
		for _, item := range structDef.Items {
			if field, ok := item.(*ast.StructField); ok {
				fields = append(fields, field.Name.Value)
			}
		}
	}

	return fields
}

func (a *Analyzer) findFunctionsBySignature(name string, argCount int, argTypes []string) []string {
	var matches []string

	// Check all standard library modules for functions with matching signatures
	modules := a.context.GetStandardModules()
	for modulePath, module := range modules {
		for funcName, funcDef := range module.Functions {
			if levenshteinDistance(name, funcName) <= 2 && len(funcName) > 1 {
				// Check if parameter count and types match
				if a.isSignatureCompatible(funcDef.Parameters, argCount, argTypes) {
					matches = append(matches, modulePath+"::{"+funcName+"}")
				}
			}
		}
	}

	// Check local functions with matching signatures
	for funcName, localFunc := range a.localFunctions {
		if levenshteinDistance(name, funcName) <= 2 && len(funcName) > 1 {
			if localFunc != nil && a.isLocalSignatureCompatible(localFunc.Params, argCount, argTypes) {
				matches = append(matches, funcName)
			}
		}
	}

	// Check imported functions with matching signatures
	importedFunctions := a.context.GetAllImportedFunctions()
	for _, funcName := range importedFunctions {
		if levenshteinDistance(name, funcName) <= 2 && len(funcName) > 1 {
			if funcDef := a.context.GetFunctionDefinition(funcName); funcDef != nil {
				if a.isSignatureCompatible(funcDef.Parameters, argCount, argTypes) {
					matches = append(matches, funcName)
				}
			}
		}
	}

	return matches
}

func (a *Analyzer) isSignatureCompatible(params []stdlib.ParameterDefinition, argCount int, argTypes []string) bool {
	// Must match parameter count
	if len(params) != argCount {
		return false
	}

	// For now, use a simpler approach: just match argument count
	// More sophisticated type matching can be added later
	return true
}

func (a *Analyzer) isLocalSignatureCompatible(params []*ast.FunctionParam, argCount int, argTypes []string) bool {
	// Must match parameter count
	if len(params) != argCount {
		return false
	}

	// For now, just match argument count for local functions too
	return true
}

func (a *Analyzer) analyzeCallContext(call *ast.CallExpr) map[string]interface{} {
	context := make(map[string]interface{})

	// Analyze argument count and types
	context["argCount"] = len(call.Args)

	argTypes := make([]string, len(call.Args))
	for i, arg := range call.Args {
		if argType := a.inferExpressionType(arg); argType != nil {
			argTypes[i] = argType.Name
		} else {
			argTypes[i] = "unknown"
		}
	}
	context["argTypes"] = argTypes

	// Check if it's used in a context that expects a return value
	context["expectsReturn"] = a.isUsedInValueContext(call)

	return context
}

func (a *Analyzer) isUsedInValueContext(expr ast.Expr) bool {
	// This is a simplified check - in a real implementation, you'd walk up the AST
	// to determine if the expression is used in a context that requires a value
	return true // For now, assume most function calls are expected to return values
}

// Simple Levenshtein distance for finding similar names
func levenshteinDistance(a, b string) int {
	if len(a) == 0 {
		return len(b)
	}
	if len(b) == 0 {
		return len(a)
	}

	if len(a) > len(b) {
		a, b = b, a
	}

	previous := make([]int, len(a)+1)
	for i := range previous {
		previous[i] = i
	}

	for i := 0; i < len(b); i++ {
		current := make([]int, len(a)+1)
		current[0] = i + 1

		for j := 0; j < len(a); j++ {
			cost := 0
			if a[j] != b[i] {
				cost = 1
			}
			current[j+1] = min3(
				current[j]+1,     // insertion
				previous[j+1]+1,  // deletion
				previous[j]+cost, // substitution
			)
		}
		previous = current
	}

	return previous[len(a)]
}

func min3(a, b, c int) int {
	if a < b {
		if a < c {
			return a
		}
		return c
	}
	if b < c {
		return b
	}
	return c
}

// isNumericLiteral checks if a string represents a numeric literal
func (a *Analyzer) isNumericLiteral(value string) bool {
	if len(value) == 0 {
		return false
	}
	// Check for decimal literals: starts with digit
	if value[0] >= '0' && value[0] <= '9' {
		return true
	}
	// Check for hexadecimal literals: starts with 0x
	if len(value) >= 2 && value[:2] == "0x" {
		return true
	}
	return false
}

// getTypeMaxValue returns the maximum value for a given numeric type
func (a *Analyzer) getTypeMaxValue(typeName string) *big.Int {
	switch typeName {
	case "U8":
		return big.NewInt(255) // 2^8 - 1
	case "U16":
		return big.NewInt(65535) // 2^16 - 1
	case "U32":
		return big.NewInt(4294967295) // 2^32 - 1
	case "U64":
		max := new(big.Int)
		max.SetString("18446744073709551615", 10) // 2^64 - 1
		return max
	case "U128":
		max := new(big.Int)
		max.SetString("340282366920938463463374607431768211455", 10) // 2^128 - 1
		return max
	case "U256":
		max := new(big.Int)
		max.SetString("115792089237316195423570985008687907853269984665640564039457584007913129639935", 10) // 2^256 - 1
		return max
	default:
		return nil
	}
}
