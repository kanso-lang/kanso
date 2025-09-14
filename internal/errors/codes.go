package errors

// Error codes for the Kanso compiler
// These codes are used in error messages and documentation
// to provide consistent error identification across the toolchain.
//
// Error code ranges:
// E0001-E0099: Semantic analysis errors
// E0100-E0199: Parser errors
// E0200-E0299: Type system errors
// E0300-E0399: Import/module errors
// E0400-E0499: Contract-specific errors
// E0500-E0599: Standard library errors
// E0600-E0699: Flow control errors
// E0700-E0799: Reserved for future use
// E0800-E0899: Warning codes
// E0900-E0999: Reserved for tooling errors

const (
	// Currently used semantic analysis errors (E0001-E0008)

	// E0001: Variable resolution errors
	ErrorUndefinedVariable = "E0001"

	// E0002: Function resolution errors
	ErrorUndefinedFunction = "E0002"

	// E0003: Type compatibility errors
	ErrorTypeMismatch = "E0003"

	// E0004: Function return type errors
	ErrorInvalidReturnType = "E0004"

	// E0005: Struct field access errors
	ErrorFieldNotFound = "E0005"

	// E0006: Struct literal validation errors
	ErrorDuplicateField = "E0006"

	// E0007: Missing required fields in struct literals
	ErrorMissingField = "E0007"

	// E0008: Binary operation type errors
	ErrorInvalidBinaryOperation = "E0008"

	// E0009: Duplicate declaration errors
	ErrorDuplicateDeclaration = "E0009"

	// E0010: Invalid attribute errors
	ErrorInvalidAttribute = "E0010"

	// E0011: Invalid reads/writes clause errors
	ErrorInvalidReadsWrites = "E0011"

	// E0012: Constructor validation errors
	ErrorInvalidConstructor = "E0012"

	// E0013: Function call argument errors
	ErrorInvalidArguments = "E0013"

	// E0014: Assignment validation errors
	ErrorInvalidAssignment = "E0014"

	// E0015: Unary/Binary operation errors
	ErrorInvalidOperation = "E0015"

	// E0016: Generic semantic error (for legacy compatibility)
	ErrorGenericSemantic = "E0016"

	// E0017: Uninitialized variable errors
	ErrorUninitializedVariable = "E0017"

	// E0018: Numeric overflow errors
	ErrorNumericOverflow = "E0018"

	// E0019: Storage access declaration errors
	ErrorStorageAccess = "E0019"

	// E0020: Void function in expression context
	ErrorVoidInExpression = "E0020"

	// E0021: Module not imported errors
	ErrorUndefinedModule = "E0021"

	// Parser errors (reserved range: E0100-E0199)
	// E0100-E0105 available for immediate use when needed

	// Type system errors (reserved range: E0200-E0299)
	// E0200-E0202 available for immediate use when needed

	// Import/module errors (reserved range: E0300-E0399)
	// E0300-E0303 available for immediate use when needed

	// Contract-specific errors (reserved range: E0400-E0499)
	// E0400-E0405 available for immediate use when needed

	// Standard library errors (reserved range: E0500-E0599)
	// E0500-E0501 available for immediate use when needed

	// Flow control errors (reserved range: E0600-E0699)

	// E0600: Missing return statement
	ErrorMissingReturn = "E0600"

	// E0601: Unreachable code
	ErrorUnreachableCode = "E0601"

	// Warning codes (reserved range: E0800-E0899)
	// E0800-E0804 available for immediate use when needed

	// Warning codes

	// W0001: Unused variable warning
	WarningUnusedVariable = "W0001"

	// W0002: Unreachable code warning
	WarningUnreachableCode = "W0002"
)

// GetErrorDescription returns a human-readable description of the error code
func GetErrorDescription(code string) string {
	switch code {
	case ErrorUndefinedVariable:
		return "Variable is used but not defined in the current scope"
	case ErrorUndefinedFunction:
		return "Function is called but not imported or defined"
	case ErrorTypeMismatch:
		return "Expression type does not match expected type"
	case ErrorInvalidReturnType:
		return "Function return value type does not match declared return type"
	case ErrorFieldNotFound:
		return "Struct field does not exist"
	case ErrorDuplicateField:
		return "Duplicate field in struct literal"
	case ErrorMissingField:
		return "Required field missing in struct literal"
	case ErrorInvalidBinaryOperation:
		return "Binary operation not supported for these types"
	case ErrorMissingReturn:
		return "Function declares return type but has no return statement"
	case ErrorUnreachableCode:
		return "Code is unreachable"
	case ErrorDuplicateDeclaration:
		return "Duplicate declaration found"
	case ErrorInvalidAttribute:
		return "Invalid or unsupported attribute"
	case ErrorInvalidReadsWrites:
		return "Invalid reads or writes clause"
	case ErrorInvalidConstructor:
		return "Invalid constructor definition"
	case ErrorInvalidArguments:
		return "Function call has invalid arguments"
	case ErrorInvalidAssignment:
		return "Invalid assignment operation"
	case ErrorInvalidOperation:
		return "Invalid unary or binary operation"
	case ErrorGenericSemantic:
		return "Semantic analysis error"
	case WarningUnusedVariable:
		return "Variable is declared but never used"
	case WarningUnreachableCode:
		return "Code is unreachable"
	default:
		return "Unknown error code"
	}
}

// IsWarning returns true if the error code represents a warning rather than an error
func IsWarning(code string) bool {
	return code >= "E0800" && code < "E0900" || code[0] == 'W'
}

// GetErrorCategory returns the category of the error based on its code
func GetErrorCategory(code string) string {
	switch {
	case code >= "E0001" && code < "E0100":
		return "Semantic Analysis"
	case code >= "E0100" && code < "E0200":
		return "Parser"
	case code >= "E0200" && code < "E0300":
		return "Type System"
	case code >= "E0300" && code < "E0400":
		return "Import/Module"
	case code >= "E0400" && code < "E0500":
		return "Contract"
	case code >= "E0500" && code < "E0600":
		return "Standard Library"
	case code >= "E0600" && code < "E0700":
		return "Flow Control"
	case code >= "E0800" && code < "E0900":
		return "Warning"
	case code[0] == 'W':
		return "Warning"
	default:
		return "Unknown"
	}
}

// GetNextAvailableErrorCode returns the next available error code in a given range
// This is useful for developers adding new error types
func GetNextAvailableErrorCode(category string) string {
	switch category {
	case "semantic":
		return "E0016" // Next available after E0015
	case "parser":
		return "E0100" // First in parser range
	case "type":
		return "E0200" // First in type system range
	case "import":
		return "E0300" // First in import range
	case "contract":
		return "E0400" // First in contract range
	case "stdlib":
		return "E0500" // First in stdlib range
	case "flow":
		return "E0602" // Next available after E0601
	case "warning":
		return "E0800" // First in warning range
	default:
		return "E0009" // Default to next semantic error
	}
}
