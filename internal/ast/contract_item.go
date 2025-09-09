package ast

type ContractItem interface {
	Node
	isContractItem()
}

func (*BadContractItem) isContractItem() {}

func (*DocComment) isContractItem() {}

func (*Comment) isContractItem() {}

func (*Attribute) isContractItem() {}

func (*Function) isContractItem() {}

func (*Struct) isContractItem() {}

func (*Use) isContractItem() {}
