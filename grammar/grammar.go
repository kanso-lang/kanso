package grammar

type Program struct {
	SourceElements []*SourceElement `@@*`
}

type SourceElement struct {
	Comment *Comment `  @@`
	Module  *Module  `| @@`
}

type DocComment struct {
	Text string `@DocComment`
}

type Comment struct {
	Text string `@Comment`
}

type Module struct {
	DocBeforeAttr *DocComment      `@@?`
	Attribute     *ModuleAttribute `@@?`
	DocAfterAttr  *DocComment      `@@?`
	Name          string           `"module" @Ident "{"`
	Uses          []*Use           `@@*`
	Structs       []*Struct        `@@*`
	Functions     []*Function      `@@*`
	Close         string           `"}"`
}

type ModuleAttribute struct {
	Name string `"#" "[" @"contract" "]"`
}

type Use struct {
	Namespaces []*Namespace  `"use" @@ ":" ":" { @@ ":" ":" } [ @@ ]`
	Imports    []*ImportItem `[ "{" @@ { "," @@ } "}" ] ";"`
}

type Namespace struct {
	Name string `@Ident`
}

type ImportItem struct {
	Name string `@Ident`
}

type Struct struct {
	DocBeforeAttr *DocComment      `@@?`
	Attribute     *StructAttribute `@@?`
	DocAfterAttr  *DocComment      `@@?`
	Name          string           `"struct" @Ident "{"`
	Fields        []*StructField   `@@* "}"`
}

type StructAttribute struct {
	Name string `"#" "[" @("storage" | "event") "]"`
}

type StructField struct {
	Name string `@Ident ":"`
	Type *Type  `@@ ","`
}

type Type struct {
	Ref      *RefType `  @@`
	Name     string   `| @Ident`
	Generics []*Type  `[ "<" @@ { "," @@ } ">" ]`
}

type RefType struct {
	And    string `"&"`
	Mut    bool   `[ @"mut" ]`
	Target *Type  `@@`
}

type Function struct {
	DocBeforeAttr *DocComment        `@@?`
	Attribute     *FunctionAttribute `@@?`
	DocAfterAttr  *DocComment        `@@?`
	Public        bool               `[ @"public" ]`
	Name          string             `"fun" @Ident "("`
	Params        []*FunctionParam   `[ @@ { "," @@ } ] ")"`
	Return        *Type              `[ ":" @@ ]`
	Reads         []*Type            `[ "reads" @@ { "," @@ } ]`
	Writes        []*Type            `[ "writes" @@ { "," @@ } ]`
	Body          *FunctionBlock     `@@`
}

type FunctionAttribute struct {
	Name string `"#" "[" @Ident "]"`
}

type FunctionParam struct {
	Name string `@Ident ":"`
	Type *Type  `@@`
}

type FunctionBlock struct {
	Statements []*Statement `"{" @@*`
	Tail       *ExprStmt    `[ @@ ] "}"`
}

type Statement struct {
	Comment    *Comment    `  @@`
	AssertStmt *AssertStmt `| @@`
	LetStmt    *LetStmt    `| @@`
	ReturnStmt *ReturnStmt `| @@`
	AssignStmt *AssignStmt `| @@`
	ExprStmt   *ExprStmt   `| @@`
}

type LetStmt struct {
	Name string `"let" @Ident "="`
	Expr *Expr  `@@ ";"`
}

type AssignStmt struct {
	Dereference bool   `[ "*" ]`
	Target      string `@Ident "="`
	Value       *Expr  `@@ ";"`
}

type ExprStmt struct {
	Expr *Expr `@@ [ ";"]`
}

type AssertStmt struct {
	Args []*Expr `"assert" "!" "(" @@ { "," @@ } ")" ";"`
}

type ReturnStmt struct {
	Expr *Expr `"return" [ @@ ] ";"`
}

type Expr struct {
	Binary *BinaryExpr `@@`
}

type BinaryExpr struct {
	Left *UnaryExpr `@@`
	Ops  []*BinOp   `{ @@ }`
}

type BinOp struct {
	Operator string     `@("||" | "&&" | "==" | "!=" | "+=" | "-=" | "*=" | "/=" | "%=" | "<" | "<=" | ">" | ">=" | "+" | "-" | "*" | "/" | "%")`
	Right    *UnaryExpr `@@`
}

type UnaryExpr struct {
	Operator *string      `[ @("!" | "-" | "*" | "&" | "|") ]`
	Mut      bool         `[ @"mut" ]`
	Value    *PostfixExpr `@@`
}

type PostfixExpr struct {
	Primary *PrimaryExpr `@@`
	Suffix  []*PostfixOp `{ @@ }`
}

type PostfixOp struct {
	Name string      `"." @Ident`
	Call *CallSuffix `[ @@ ]`
}

type CallSuffix struct {
	Args []*Expr `"(" [ @@ { "," @@ } ] ")"`
}

type MethodCall struct {
	Name string  `"." @Ident`
	Args []*Expr `"(" [ @@ { "," @@ } ] ")"`
}

type PrimaryExpr struct {
	Call   *CallExpr          `  @@`
	Struct *StructLiteralExpr `| @@`
	Number *string            `| @Integer`
	Ident  *string            `| @Ident`
	Parens *Expr              `| "(" @@ ")"`
}

type FieldAccess struct {
	Name string `"." @Ident`
}

type CallExpr struct {
	Callee  *CalleePath `@@`
	Generic []*Type     `[ "<" @@ { "," @@ } ">" ]`
	Args    []*Expr     `"(" [ @@ { "," @@ } ] ")"`
}

type CalleePath struct {
	Parts []string `@Ident { ":" ":" @Ident }`
}

type StructLiteralExpr struct {
	Name   string                `@Ident "{"`
	Fields []*StructLiteralField `@@ { "," @@ } [ "," ] "}"`
}

type StructLiteralField struct {
	Name  string `@Ident`
	Value *Expr  `[ ":" @@ ]`
}

type StructFieldFull struct {
	Name  string `@Ident ":"`
	Value *Expr  `@@`
}
