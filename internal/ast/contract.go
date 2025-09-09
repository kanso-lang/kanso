package ast

// Contract represents a Kanso contract (the entire source file)
// Example: "// License comment\ncontract ERC20 { use std::evm; struct Transfer { ... } fn create() { ... } }"
type Contract struct {
	Pos             Position
	EndPos          Position
	LeadingComments []ContractItem // Comments before the contract declaration
	Name            Ident
	Items           []ContractItem // Items inside the contract block
	metadata        *Metadata
}

// Position tracks location information for error reporting and tooling
type Position struct {
	Filename string
	Offset   int
	Line     int
	Column   int
}

// Ident represents any identifier like variable names, type names, etc.
// Example: "ERC20", "balanceOf", "owner", "amount"
type Ident struct {
	Pos      Position
	EndPos   Position
	Value    string
	metadata *Metadata
}

// BadContractItem represents parse errors in contract-level items
type BadContractItem struct {
	Bad      BadNode
	metadata *Metadata
}

// BadModuleItem represents parse errors in module-level items (legacy)
type BadModuleItem struct {
	Bad      BadNode
	metadata *Metadata
}

// BadExpr represents parse errors in expressions
type BadExpr struct {
	Bad      BadNode
	metadata *Metadata
}

// BadNode contains error information for failed parsing
type BadNode struct {
	Pos      Position
	EndPos   Position
	Message  string
	Details  []string
	metadata *Metadata
}

// DocComment represents documentation comments
// Example: "/// Returns the balance of an account"
type DocComment struct {
	Pos      Position
	EndPos   Position
	Text     string
	metadata *Metadata
}

// Comment represents regular comments
// Example: "// This is a comment"
type Comment struct {
	Pos      Position
	EndPos   Position
	Text     string
	metadata *Metadata
}

// Attribute represents attributes like #[storage], #[event], #[create]
// Example: "#[storage]", "#[event]", "#[create]"
type Attribute struct {
	Pos      Position
	EndPos   Position
	Name     string
	metadata *Metadata
}

// Use represents import statements
// Example: "use std::evm::{sender, emit};"
type Use struct {
	Pos        Position
	EndPos     Position
	Namespaces []*Namespace
	Imports    []*ImportItem
	metadata   *Metadata
}

// Namespace represents namespace parts in use statements
// Example: "std", "evm" in "use std::evm::{sender, emit};"
type Namespace struct {
	Pos      Position
	EndPos   Position
	Name     Ident
	metadata *Metadata
}

// ImportItem represents individual imported items
// Example: "sender", "emit" in "use std::evm::{sender, emit};"
type ImportItem struct {
	Pos      Position
	EndPos   Position
	Name     Ident
	metadata *Metadata
}

// Struct represents struct declarations
// Example: "struct State { balances: Slots<Address, U256>, total_supply: U256 }"
type Struct struct {
	Pos        Position
	EndPos     Position
	Attribute  *Attribute
	DocComment *DocComment
	Name       Ident
	Items      []StructItem
	metadata   *Metadata
}

// StructField represents individual fields within a struct
// Example: "balances: Slots<Address, U256>", "total_supply: U256"
type StructField struct {
	Pos          Position
	EndPos       Position
	Name         Ident
	VariableType *VariableType
	metadata     *Metadata
}

// VariableType represents type specifications
// Example: "U256", "Address", "Slots<Address, U256>", "(Address, U256)"
type VariableType struct {
	Pos           Position
	EndPos        Position
	Name          Ident
	Generics      []*VariableType
	TupleElements []*VariableType // For tuple types like (Address, U256)
	metadata      *Metadata
}

// Function represents function declarations
// Example: "ext fn balanceOf(owner: Address) -> U256 reads(State) { ... }"
type Function struct {
	Pos        Position
	EndPos     Position
	Attribute  *Attribute
	DocComment *DocComment
	External   bool
	Name       Ident
	Params     []*FunctionParam
	Return     *VariableType
	Reads      []Ident
	Writes     []Ident
	Body       *FunctionBlock
	metadata   *Metadata
}

// FunctionParam represents function parameters
// Example: "owner: Address", "amount: U256"
type FunctionParam struct {
	Pos      Position
	EndPos   Position
	Name     Ident
	Type     *VariableType
	metadata *Metadata
}

// FunctionBlock represents the body of a function
// Example: "{ let balance = State.balances[owner]; balance }"
type FunctionBlock struct {
	Pos      Position
	EndPos   Position
	Items    []FunctionBlockItem
	TailExpr *ExprStmt // optional final expr without semicolon
	metadata *Metadata
}

// ExprStmt represents expression statements
// Example: "do_transfer(from, to, amount);", "State.balances[owner]"
type ExprStmt struct {
	Pos       Position
	EndPos    Position
	Expr      Expr
	Semicolon bool // true if a `;` was present
	metadata  *Metadata
}

// ReturnStmt represents return statements
// Example: "return balance;", "return;"
type ReturnStmt struct {
	Pos      Position
	EndPos   Position
	Value    Expr // nil if plain `return;`
	metadata *Metadata
}

// LetStmt represents variable declarations
// Example: "let balance = State.balances[owner];", "let mut counter = 0;"
type LetStmt struct {
	Pos      Position
	EndPos   Position
	Mut      bool // true for "let mut"
	Name     Ident
	Expr     Expr
	metadata *Metadata
}

// AssignStmt represents assignment statements
// Example: "State.balances[owner] = amount;", "total_supply += amount;"
type AssignStmt struct {
	Pos      Position
	EndPos   Position
	Target   Expr
	Operator AssignType
	Value    Expr
	metadata *Metadata
}

// RequireStmt represents require statements
// Example: "require!(amount > 0, errors::InvalidAmount);"
type RequireStmt struct {
	Pos      Position
	EndPos   Position
	Args     []Expr
	metadata *Metadata
}

// BinaryExpr represents binary operations
// Example: "amount + fee", "balance >= amount", "sender() != to"
type BinaryExpr struct {
	Pos      Position
	EndPos   Position
	Op       string
	Left     Expr
	Right    Expr
	metadata *Metadata
}

// UnaryExpr represents unary operations
// Example: "-amount", "!condition", "&mut balance"
type UnaryExpr struct {
	Pos      Position
	EndPos   Position
	Op       string
	Value    Expr
	Mut      bool
	metadata *Metadata
}

// CallExpr represents function calls
// Example: "sender()", "do_transfer(from, to, amount)", "emit(Transfer { ... })"
type CallExpr struct {
	Pos      Position
	EndPos   Position
	Callee   Expr
	Generic  []VariableType
	Args     []Expr
	metadata *Metadata
}

// FieldAccessExpr represents field access
// Example: "State.balances", "State.total_supply"
type FieldAccessExpr struct {
	Pos      Position
	EndPos   Position
	Target   Expr
	Field    string
	metadata *Metadata
}

// IndexExpr represents array/map indexing
// Example: "State.balances[owner]", "State.allowances[(from, spender)]"
type IndexExpr struct {
	Pos      Position
	EndPos   Position
	Target   Expr
	Index    Expr
	metadata *Metadata
}

// StructLiteralExpr represents struct literals
// Example: "Transfer { from: sender(), to: recipient, value: amount }"
type StructLiteralExpr struct {
	Pos      Position
	EndPos   Position
	Name     string
	Type     *CalleePath
	Fields   []StructLiteralField
	metadata *Metadata
}

// LiteralExpr represents literal values
// Example: "100", "0x42", "\"hello\"", "true"
type LiteralExpr struct {
	Pos      Position
	EndPos   Position
	Value    string
	metadata *Metadata
}

// IdentExpr represents simple identifiers
// Example: "amount", "owner", "State"
type IdentExpr struct {
	Pos      Position
	EndPos   Position
	Name     string
	metadata *Metadata
}

// CalleePath represents module paths and qualified names
// Example: "errors::InvalidAmount", "std::evm::sender"
type CalleePath struct {
	Pos      Position
	EndPos   Position
	Parts    []Ident
	metadata *Metadata
}

// StructLiteralField represents fields in struct literals
// Example: "from: sender()", "value: amount" in "Transfer { from: sender(), value: amount }"
type StructLiteralField struct {
	Pos      Position
	EndPos   Position
	Name     Ident
	Value    Expr
	metadata *Metadata
}

// ParenExpr represents parenthesized expressions
// Example: "(amount + fee)", "(sender() != to)"
type ParenExpr struct {
	Pos      Position
	EndPos   Position
	Value    Expr
	metadata *Metadata
}

// TupleExpr represents tuple expressions
// Example: "(from, sender())", "(42, true, \"test\")"
type TupleExpr struct {
	Pos      Position
	EndPos   Position
	Elements []Expr
	metadata *Metadata
}
