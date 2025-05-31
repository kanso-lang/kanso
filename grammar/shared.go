package grammar

import (
	"github.com/alecthomas/participle/v2/lexer"
)

type PosIdent struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Value  string `@Ident`
}

type DocComment struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Text   string `@DocComment`
}

type Comment struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Text   string `@Comment`
}

type Module struct {
	Pos           lexer.Position
	EndPos        lexer.Position
	DocBeforeAttr *DocComment      `@@?`
	Attribute     *ModuleAttribute `@@?`
	DocAfterAttr  *DocComment      `@@?`
	Name          PosIdent         `"module" @@ "{"`
	Uses          []*Use           `@@*`
	Structs       []*Struct        `@@*`
	Functions     []*Function      `@@*`
	Close         string           `"}"`
}

type ModuleAttribute struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   string `"#" "[" @"contract" "]"`
}

type Use struct {
	Pos        lexer.Position
	EndPos     lexer.Position
	Tokens     []lexer.Token
	Namespaces []*Namespace  `"use" @@ ":" ":" { @@ ":" ":" } [ @@ ]`
	Imports    []*ImportItem `[ "{" @@ { "," @@ } "}" ] ";"`
}

type Namespace struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Name   PosIdent `@@`
}

type ImportItem struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent `@@`
}

type Struct struct {
	Pos           lexer.Position
	EndPos        lexer.Position
	Tokens        []lexer.Token
	DocBeforeAttr *DocComment      `@@?`
	Attribute     *StructAttribute `@@?`
	DocAfterAttr  *DocComment      `@@?`
	Name          PosIdent         `"struct" @@ "{"`
	Fields        []*StructField   `@@* "}"`
}

type StructAttribute struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   string `"#" "[" @("storage" | "event") "]"`
}

type StructField struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent `@@ ":"`
	Type   *Type    `@@ ","`
}

type Type struct {
	Pos      lexer.Position
	EndPos   lexer.Position
	Tokens   []lexer.Token
	Ref      *RefType `  @@`
	Name     PosIdent `| @@`
	Generics []*Type  `[ "<" @@ { "," @@ } ">" ]`
}

type RefType struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	And    string `"&"`
	Mut    bool   `[ @"mut" ]`
	Target *Type  `@@`
}

type Function struct {
	Pos           lexer.Position
	EndPos        lexer.Position
	DocBeforeAttr *DocComment        `@@?`
	Attribute     *FunctionAttribute `@@?`
	DocAfterAttr  *DocComment        `@@?`
	Public        bool               `[ @"public" ]`
	Name          PosIdent           `"fun" @@ "("`
	Params        []*FunctionParam   `[ @@ { "," @@ } ] ")"`
	Return        *Type              `[ ":" @@ ]`
	Reads         []*Type            `[ "reads" @@ { "," @@ } ]`
	Writes        []*Type            `[ "writes" @@ { "," @@ } ]`
	Body          *FunctionBlock     `@@`
}

type FunctionAttribute struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent `"#" "[" @@ "]"`
}

type FunctionParam struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent `@@ ":"`
	Type   *Type    `@@`
}

type FunctionBlock struct {
	Pos        lexer.Position
	EndPos     lexer.Position
	Statements []*Statement `"{" @@*`
	Tail       *ExprStmt    `[ @@ ] "}"`
}

type LetStmt struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent `"let" @@ "="`
	Expr   *Expr    `@@ ";"`
}

type AssignStmt struct {
	Pos         lexer.Position
	EndPos      lexer.Position
	Tokens      []lexer.Token
	Dereference bool     `[ "*" ]`
	Target      PosIdent `@@ "="`
	Value       *Expr    `@@ ";"`
}

type ExprStmt struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Expr   *Expr `@@ [ ";"]`
}

type AssertStmt struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Args   []*Expr `"assert" "!" "(" @@ { "," @@ } ")" ";"`
}

type ReturnStmt struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Expr   *Expr `"return" [ @@ ] ";"`
}

type Expr struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Binary *BinaryExpr `@@`
}

type BinaryExpr struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Left   *UnaryExpr `@@`
	Ops    []*BinOp   `{ @@ }`
}

type BinOp struct {
	Pos      lexer.Position
	EndPos   lexer.Position
	Tokens   []lexer.Token
	Operator string     `@("||" | "&&" | "==" | "!=" | "+=" | "-=" | "*=" | "/=" | "%=" | "<" | "<=" | ">" | ">=" | "+" | "-" | "*" | "/" | "%")`
	Right    *UnaryExpr `@@`
}

type UnaryExpr struct {
	Pos      lexer.Position
	EndPos   lexer.Position
	Tokens   []lexer.Token
	Operator string       `[ @("!" | "-" | "*" | "&" | "|") ]`
	Mut      bool         `[ @"mut" ]`
	Value    *PostfixExpr `@@`
}

type PostfixExpr struct {
	Pos     lexer.Position
	EndPos  lexer.Position
	Tokens  []lexer.Token
	Primary *PrimaryExpr `@@`
	Suffix  []*PostfixOp `{ @@ }`
}

type PostfixOp struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent    `"." @@`
	Call   *CallSuffix `[ @@ ]`
}

type CallSuffix struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Args   []*Expr `"(" [ @@ { "," @@ } ] ")"`
}

type MethodCall struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent `"." @@`
	Args   []*Expr  `"(" [ @@ { "," @@ } ] ")"`
}

type PrimaryExpr struct {
	Call   *CallExpr          `  @@`
	Struct *StructLiteralExpr `| @@`
	Number *string            `| @Integer`
	Ident  *PosIdent          `| @@`
	Parens *Expr              `| "(" @@ ")"`
}

type FieldAccess struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Name   PosIdent `"." @@`
}

type CallExpr struct {
	Pos     lexer.Position
	EndPos  lexer.Position
	Tokens  []lexer.Token
	Callee  *CalleePath `@@`
	Generic []*Type     `[ "<" @@ { "," @@ } ">" ]`
	Args    []*Expr     `"(" [ @@ { "," @@ } ] ")"`
}

type CalleePath struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Parts  []PosIdent `@@ { ":" ":" @@ }`
}

type StructLiteralExpr struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent              `@@ "{"`
	Fields []*StructLiteralField `@@ { "," @@ } [ "," ] "}"`
}

type StructLiteralField struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent `@@`
	Value  *Expr    `[ ":" @@ ]`
}

type StructFieldFull struct {
	Pos    lexer.Position
	EndPos lexer.Position
	Tokens []lexer.Token
	Name   PosIdent `@@ ":"`
	Value  *Expr    `@@`
}
