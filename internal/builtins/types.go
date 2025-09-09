package builtins

// BuiltinType represents the built-in types in the Kanso language
type BuiltinType string

const (
	// Unsigned integers
	U8   BuiltinType = "U8"
	U16  BuiltinType = "U16"
	U32  BuiltinType = "U32"
	U64  BuiltinType = "U64"
	U128 BuiltinType = "U128"
	U256 BuiltinType = "U256"

	// Other primitives
	Bool    BuiltinType = "Bool"
	Address BuiltinType = "Address"
)

// BuiltinTypes contains all valid built-in types
var BuiltinTypes = map[string]bool{
	// Unsigned integers
	string(U8):   true,
	string(U16):  true,
	string(U32):  true,
	string(U64):  true,
	string(U128): true,
	string(U256): true,

	// Other primitives
	string(Bool):    true,
	string(Address): true,
}

// IsBuiltinType checks if a type name is a built-in type
func IsBuiltinType(typeName string) bool {
	return BuiltinTypes[typeName]
}

// IsIntegerType checks if a type is an unsigned integer type
func IsIntegerType(typeName string) bool {
	switch BuiltinType(typeName) {
	case U8, U16, U32, U64, U128, U256:
		return true
	default:
		return false
	}
}
