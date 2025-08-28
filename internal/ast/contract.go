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
	Pos    Position
	EndPos Position
	Value  string
}

type BadContractItem struct {
	Bad BadNode
}

type BadModuleItem struct {
	Bad BadNode
}

type BadExpr struct {
	Bad BadNode
}

type BadNode struct {
	Pos     Position
	EndPos  Position
	Message string
	Details []string
}

type DocComment struct {
	Pos    Position
	EndPos Position
	Text   string
}

type Comment struct {
	Pos    Position
	EndPos Position
	Text   string
}

type Module struct {
	Pos         Position
	EndPos      Position
	Attributes  []Attribute
	Name        Ident
	ModuleItems []ModuleItem
}

type Attribute struct {
	Pos    Position
	EndPos Position
	Name   string
}

type Use struct {
	Pos        Position
	EndPos     Position
	Namespaces []*Namespace
	Imports    []*ImportItem
}

type Namespace struct {
	Pos    Position
	EndPos Position
	Name   Ident
}

type ImportItem struct {
	Pos    Position
	EndPos Position
	Name   Ident
}

type Struct struct {
	Pos       Position
	EndPos    Position
	Attribute *Attribute
	Name      Ident
	Items     []StructItem
}

type StructField struct {
	Pos          Position
	EndPos       Position
	Name         Ident
	VariableType *VariableType
}

type VariableType struct {
	Pos      Position
	EndPos   Position
	Ref      *RefVariableType
	Name     Ident
	Generics []*VariableType
}

type RefVariableType struct {
	Pos    Position
	EndPos Position
	And    bool
	Mut    bool
	Target *VariableType
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
}

type FunctionParam struct {
	Pos    Position
	EndPos Position
	Name   Ident
	Type   *VariableType
}

type FunctionBlock struct {
	Pos      Position
	EndPos   Position
	Items    []FunctionBlockItem
	TailExpr *ExprStmt // optional final expr without semicolon
}

type ExprStmt struct {
	Pos       Position
	EndPos    Position
	Expr      Expr
	Semicolon bool // true if a `;` was present
}

type ReturnStmt struct {
	Pos    Position
	EndPos Position
	Value  Expr // nil if plain `return;`
}

type LetStmt struct {
	Pos    Position
	EndPos Position
	Name   Ident
	Expr   Expr
}

type AssignStmt struct {
	Pos      Position
	EndPos   Position
	Target   Expr
	Operator AssignType
	Value    Expr
}

type AssertStmt struct {
	Pos    Position
	EndPos Position
	Args   []Expr
}

type BinaryExpr struct {
	Pos    Position
	EndPos Position
	Op     string
	Left   Expr
	Right  Expr
}

type UnaryExpr struct {
	Pos    Position
	EndPos Position
	Op     string
	Value  Expr
	Mut    bool
}

type CallExpr struct {
	Pos     Position
	EndPos  Position
	Callee  Expr
	Generic []VariableType
	Args    []Expr
}

type FieldAccessExpr struct {
	Pos    Position
	EndPos Position
	Target Expr
	Field  string
}

type StructLiteralExpr struct {
	Pos    Position
	EndPos Position
	Name   string
	Type   *CalleePath
	Fields []StructLiteralField
}

type LiteralExpr struct {
	Pos    Position
	EndPos Position
	Value  string
}

type IdentExpr struct {
	Pos    Position
	EndPos Position
	Name   string
}

type CalleePath struct {
	Pos    Position
	EndPos Position
	Parts  []Ident
}

type StructLiteralField struct {
	Pos    Position
	EndPos Position
	Name   Ident
	Value  Expr
}

type ParenExpr struct {
	Pos    Position
	EndPos Position
	Value  Expr
}
