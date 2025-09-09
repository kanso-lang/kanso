package ast

type Expr interface {
	Node
	isExpr()
}

func (*BadExpr) isExpr() {}

func (*BinaryExpr) isExpr() {}

func (*UnaryExpr) isExpr() {}

func (*CallExpr) isExpr() {}

func (*FieldAccessExpr) isExpr() {}

func (*IndexExpr) isExpr() {}

func (*StructLiteralExpr) isExpr() {}

func (*LiteralExpr) isExpr() {}

func (*IdentExpr) isExpr() {}

func (*CalleePath) isExpr() {}

func (*StructLiteralField) isExpr() {}

func (*ParenExpr) isExpr() {}

func (*TupleExpr) isExpr() {}
