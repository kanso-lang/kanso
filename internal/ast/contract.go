package ast

type Contract struct {
	ContractItems []ContractItem
}

type Position struct {
	Filename string
	Offset   int
	Line     int
	Column   int
}

type Ident struct {
	Pos      Position
	EndPos   Position
	Value    string
	metadata *Metadata
}

type BadContractItem struct {
	Bad      BadNode
	metadata *Metadata
}

type BadModuleItem struct {
	Bad      BadNode
	metadata *Metadata
}

type BadExpr struct {
	Bad      BadNode
	metadata *Metadata
}

type BadNode struct {
	Pos      Position
	EndPos   Position
	Message  string
	Details  []string
	metadata *Metadata
}

type DocComment struct {
	Pos      Position
	EndPos   Position
	Text     string
	metadata *Metadata
}

type Comment struct {
	Pos      Position
	EndPos   Position
	Text     string
	metadata *Metadata
}

type Module struct {
	Pos         Position
	EndPos      Position
	Attributes  []Attribute
	Name        Ident
	ModuleItems []ModuleItem
	metadata    *Metadata
}

type Attribute struct {
	Pos      Position
	EndPos   Position
	Name     string
	metadata *Metadata
}

type Use struct {
	Pos        Position
	EndPos     Position
	Namespaces []*Namespace
	Imports    []*ImportItem
	metadata   *Metadata
}

type Namespace struct {
	Pos      Position
	EndPos   Position
	Name     Ident
	metadata *Metadata
}

type ImportItem struct {
	Pos      Position
	EndPos   Position
	Name     Ident
	metadata *Metadata
}

type Struct struct {
	Pos       Position
	EndPos    Position
	Attribute *Attribute
	Name      Ident
	Items     []StructItem
	metadata  *Metadata
}

type StructField struct {
	Pos          Position
	EndPos       Position
	Name         Ident
	VariableType *VariableType
	metadata     *Metadata
}

type VariableType struct {
	Pos      Position
	EndPos   Position
	Ref      *RefVariableType
	Name     Ident
	Generics []*VariableType
	metadata *Metadata
}

type RefVariableType struct {
	Pos      Position
	EndPos   Position
	And      bool
	Mut      bool
	Target   *VariableType
	metadata *Metadata
}

type Function struct {
	Pos       Position
	EndPos    Position
	Attribute *Attribute
	Public    bool
	Name      Ident
	Params    []*FunctionParam
	Return    *VariableType
	Reads     []Ident
	Writes    []Ident
	Body      *FunctionBlock
	metadata  *Metadata
}

type FunctionParam struct {
	Pos      Position
	EndPos   Position
	Name     Ident
	Type     *VariableType
	metadata *Metadata
}

type FunctionBlock struct {
	Pos      Position
	EndPos   Position
	Items    []FunctionBlockItem
	TailExpr *ExprStmt // optional final expr without semicolon
	metadata *Metadata
}

type ExprStmt struct {
	Pos       Position
	EndPos    Position
	Expr      Expr
	Semicolon bool // true if a `;` was present
	metadata  *Metadata
}

type ReturnStmt struct {
	Pos      Position
	EndPos   Position
	Value    Expr // nil if plain `return;`
	metadata *Metadata
}

type LetStmt struct {
	Pos      Position
	EndPos   Position
	Name     Ident
	Expr     Expr
	metadata *Metadata
}

type AssignStmt struct {
	Pos      Position
	EndPos   Position
	Target   Expr
	Operator AssignType
	Value    Expr
	metadata *Metadata
}

type AssertStmt struct {
	Pos      Position
	EndPos   Position
	Args     []Expr
	metadata *Metadata
}

type BinaryExpr struct {
	Pos      Position
	EndPos   Position
	Op       string
	Left     Expr
	Right    Expr
	metadata *Metadata
}

type UnaryExpr struct {
	Pos      Position
	EndPos   Position
	Op       string
	Value    Expr
	Mut      bool
	metadata *Metadata
}

type CallExpr struct {
	Pos      Position
	EndPos   Position
	Callee   Expr
	Generic  []VariableType
	Args     []Expr
	metadata *Metadata
}

type FieldAccessExpr struct {
	Pos      Position
	EndPos   Position
	Target   Expr
	Field    string
	metadata *Metadata
}

type StructLiteralExpr struct {
	Pos      Position
	EndPos   Position
	Name     string
	Type     *CalleePath
	Fields   []StructLiteralField
	metadata *Metadata
}

type LiteralExpr struct {
	Pos      Position
	EndPos   Position
	Value    string
	metadata *Metadata
}

type IdentExpr struct {
	Pos      Position
	EndPos   Position
	Name     string
	metadata *Metadata
}

type CalleePath struct {
	Pos      Position
	EndPos   Position
	Parts    []Ident
	metadata *Metadata
}

type StructLiteralField struct {
	Pos      Position
	EndPos   Position
	Name     Ident
	Value    Expr
	metadata *Metadata
}

type ParenExpr struct {
	Pos      Position
	EndPos   Position
	Value    Expr
	metadata *Metadata
}
