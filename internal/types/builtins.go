package types

import "kanso/internal/builtins"

// Re-export builtins for backward compatibility
type BuiltinType = builtins.BuiltinType

const (
	// Re-export built-in type constants
	U8   = builtins.U8
	U16  = builtins.U16
	U32  = builtins.U32
	U64  = builtins.U64
	U128 = builtins.U128
	U256 = builtins.U256

	Bool    = builtins.Bool
	Address = builtins.Address
)

// BuiltinTypes contains all valid built-in types
var BuiltinTypes = builtins.BuiltinTypes

// IsBuiltinType checks if a type name is a built-in type
func IsBuiltinType(typeName string) bool {
	return builtins.IsBuiltinType(typeName)
}

// IsIntegerType checks if a type is an unsigned integer type
func IsIntegerType(typeName string) bool {
	return builtins.IsIntegerType(typeName)
}
