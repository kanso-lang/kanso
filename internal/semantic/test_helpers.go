package semantic

// Test helper functions for filtering development-time errors that shouldn't interfere with testing other functionality.

// FilterUnusedVariables removes unused variable errors.
func FilterUnusedVariables(errors []SemanticError) []SemanticError {
	var filtered []SemanticError

	for _, err := range errors {
		if !isUnusedVariableError(err) {
			filtered = append(filtered, err)
		}
	}

	return filtered
}

// FilterUnusedFunctions removes unused function errors.
func FilterUnusedFunctions(errors []SemanticError) []SemanticError {
	var filtered []SemanticError

	for _, err := range errors {
		if !isUnusedFunctionError(err) {
			filtered = append(filtered, err)
		}
	}

	return filtered
}

// FilterMutableNeverModified removes "mutable but never modified" errors.
func FilterMutableNeverModified(errors []SemanticError) []SemanticError {
	var filtered []SemanticError

	for _, err := range errors {
		if !isMutableNeverModifiedError(err) {
			filtered = append(filtered, err)
		}
	}

	return filtered
}

// FilterModifiedButUnused removes "modified but never used" errors.
func FilterModifiedButUnused(errors []SemanticError) []SemanticError {
	var filtered []SemanticError

	for _, err := range errors {
		if !isModifiedButUnusedError(err) {
			filtered = append(filtered, err)
		}
	}

	return filtered
}

// FilterUnreachableCode removes unreachable code warnings.
func FilterUnreachableCode(errors []SemanticError) []SemanticError {
	var filtered []SemanticError

	for _, err := range errors {
		if !isUnreachableCodeError(err) {
			filtered = append(filtered, err)
		}
	}

	return filtered
}

// FilterAllUnusedErrors removes all unused-related errors.
func FilterAllUnusedErrors(errors []SemanticError) []SemanticError {
	var filtered []SemanticError

	for _, err := range errors {
		if !isAnyUnusedError(err) {
			filtered = append(filtered, err)
		}
	}

	return filtered
}

// FilterDevelopmentWarnings removes all development-time warnings.
func FilterDevelopmentWarnings(errors []SemanticError) []SemanticError {
	var filtered []SemanticError

	for _, err := range errors {
		if !isDevelopmentWarning(err) {
			filtered = append(filtered, err)
		}
	}

	return filtered
}

func isUnusedVariableError(err SemanticError) bool {
	msg := err.Message
	return containsSubstring(msg, "variable") && containsSubstring(msg, "never used")
}

func isUnusedFunctionError(err SemanticError) bool {
	msg := err.Message
	return containsSubstring(msg, "function") && containsSubstring(msg, "never used")
}

func isMutableNeverModifiedError(err SemanticError) bool {
	msg := err.Message
	return containsSubstring(msg, "mutable") && containsSubstring(msg, "never modified")
}

func isModifiedButUnusedError(err SemanticError) bool {
	msg := err.Message
	return containsSubstring(msg, "modified") && containsSubstring(msg, "never used")
}

func isUnreachableCodeError(err SemanticError) bool {
	msg := err.Message
	return containsSubstring(msg, "unreachable code")
}

func isAnyUnusedError(err SemanticError) bool {
	return isUnusedVariableError(err) ||
		isUnusedFunctionError(err) ||
		isMutableNeverModifiedError(err) ||
		isModifiedButUnusedError(err)
}

func isDevelopmentWarning(err SemanticError) bool {
	return isAnyUnusedError(err) || isUnreachableCodeError(err)
}

func containsSubstring(s, substr string) bool {
	if len(substr) > len(s) {
		return false
	}
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
