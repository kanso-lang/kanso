package ast

type ContractItem interface {
	Node
	isContractItem()
}

func (*BadContractItem) isContractItem() {}

func (*DocComment) isContractItem() {}

func (*Comment) isContractItem() {}

func (*Module) isContractItem() {}

func (*Attribute) isContractItem() {}
