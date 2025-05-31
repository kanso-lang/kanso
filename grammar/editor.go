//go:build editor
// +build editor

package grammar

type AST struct {
	SourceElements []*SourceElement `@@*`
}

type SourceElement struct {
	Comment *Comment `  @@`
	Module  *Module  `| @@`
}

type Statement struct {
	Comment    *Comment    `  @@`
	AssertStmt *AssertStmt `| @@`
	LetStmt    *LetStmt    `| @@`
	ReturnStmt *ReturnStmt `| @@`
	AssignStmt *AssignStmt `| @@`
	ExprStmt   *ExprStmt   `| @@`
	Error      *ErrorNode  `| @@`
}

type ErrorNode struct {
	Unexpected []string `(@("." | "," | ";" | @Ident)) +`
}
