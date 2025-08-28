package ast

type FunctionBlockItem interface {
	Node
	isBlockItem()
}

func (*LetStmt) isBlockItem()    {}
func (*AssignStmt) isBlockItem() {}
func (*AssertStmt) isBlockItem() {}
func (*ReturnStmt) isBlockItem() {}
func (*ExprStmt) isBlockItem()   {}
func (*Comment) isBlockItem()    {}
