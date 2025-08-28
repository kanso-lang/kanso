package ast

type StructItem interface {
	Node
	isStructItem()
}

func (*Comment) isStructItem() {}

func (*StructField) isStructItem() {}
