package ast

type ModuleItem interface {
	Node
	isModuleItem()
}

func (*DocComment) isModuleItem() {}

func (*Comment) isModuleItem() {}

func (*BadModuleItem) isModuleItem() {}

func (*Attribute) isModuleItem() {}

func (*Use) isModuleItem() {}

func (*Struct) isModuleItem() {}

func (*Function) isModuleItem() {}
