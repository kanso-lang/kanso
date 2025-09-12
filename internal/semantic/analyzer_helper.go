package semantic

import "kanso/internal/ast"

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
	// TODO: Implement imported function lookup

	return similar
}

func (a *Analyzer) findPossibleImports(name string) []string {
	// This would check the standard library for functions with similar names
	// and suggest the appropriate import statements
	var imports []string

	// TODO: Implement standard library function lookup

	return imports
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
	// Simple check: starts with digit
	return value[0] >= '0' && value[0] <= '9'
}
