package ast

// UpdateBytecodeMapping updates the bytecode mapping for a node
func UpdateBytecodeMapping(node Node, startAddr, endAddr uint32, instructions []InstructionMapping) {
	if node == nil {
		return
	}

	meta := node.GetMetadata()
	if meta == nil {
		return
	}

	if meta.CompilationInfo == nil {
		meta.CompilationInfo = &CompilationMetadata{}
	}

	meta.CompilationInfo.BytecodeRange = &BytecodeRange{
		StartAddress: startAddr,
		EndAddress:   endAddr,
		Instructions: instructions,
	}
}

// UpdateIRMapping updates the IR mapping for a node
func UpdateIRMapping(node Node, irID uint32) {
	if node == nil {
		return
	}

	meta := node.GetMetadata()
	if meta == nil {
		return
	}

	if meta.CompilationInfo == nil {
		meta.CompilationInfo = &CompilationMetadata{}
	}

	meta.CompilationInfo.IRID = irID
}

// UpdateTypeInfo updates the type information for a node
func UpdateTypeInfo(node Node, typeName string, generics []string, sizeBytes uint32, isRef, isMut bool) {
	if node == nil {
		return
	}

	meta := node.GetMetadata()
	if meta == nil {
		return
	}

	if meta.CompilationInfo == nil {
		meta.CompilationInfo = &CompilationMetadata{}
	}

	meta.CompilationInfo.TypeInfo = &TypeMetadata{
		TypeName:    typeName,
		Generics:    generics,
		SizeBytes:   sizeBytes,
		IsReference: isRef,
		IsMutable:   isMut,
	}
}

// MarkOptimizedOut marks a node as optimized out
func MarkOptimizedOut(node Node, pass string, inlinedFrom *NodeID, constantFolded bool, originalValue string) {
	if node == nil {
		return
	}

	meta := node.GetMetadata()
	if meta == nil {
		return
	}

	if meta.CompilationInfo == nil {
		meta.CompilationInfo = &CompilationMetadata{}
	}

	if meta.CompilationInfo.OptimizationInfo == nil {
		meta.CompilationInfo.OptimizationInfo = &OptimizationInfo{}
	}

	optInfo := meta.CompilationInfo.OptimizationInfo
	optInfo.OptimizedOut = true
	optInfo.OptimizationPasses = append(optInfo.OptimizationPasses, pass)
	if inlinedFrom != nil {
		optInfo.InlinedFrom = inlinedFrom
	}
	optInfo.ConstantFolded = constantFolded
	if originalValue != "" {
		optInfo.OriginalValue = originalValue
	}
}

// CreateInstructionMapping creates an instruction mapping
func CreateInstructionMapping(pos Position, addr uint32, instruction, operand string) InstructionMapping {
	return InstructionMapping{
		SourcePos:   pos,
		Address:     addr,
		Instruction: instruction,
		OperandInfo: operand,
	}
}

// GetSourceMapping returns source-to-bytecode mapping for debugging
func GetSourceMapping(nodes []Node) map[uint32]Position {
	mapping := make(map[uint32]Position)

	for _, node := range nodes {
		if node == nil {
			continue
		}

		meta := node.GetMetadata()
		if meta == nil || meta.CompilationInfo == nil || meta.CompilationInfo.BytecodeRange == nil {
			continue
		}

		bcRange := meta.CompilationInfo.BytecodeRange
		for _, instr := range bcRange.Instructions {
			mapping[instr.Address] = instr.SourcePos
		}
	}

	return mapping
}

// GetReverseMapping returns bytecode-to-source mapping for debugging
func GetReverseMapping(nodes []Node) map[Position][]uint32 {
	mapping := make(map[Position][]uint32)

	for _, node := range nodes {
		if node == nil {
			continue
		}

		meta := node.GetMetadata()
		if meta == nil || meta.CompilationInfo == nil || meta.CompilationInfo.BytecodeRange == nil {
			continue
		}

		bcRange := meta.CompilationInfo.BytecodeRange
		for _, instr := range bcRange.Instructions {
			mapping[instr.SourcePos] = append(mapping[instr.SourcePos], instr.Address)
		}
	}

	return mapping
}

// CollectAllNodes performs a deep traversal to collect all nodes with metadata
func CollectAllNodes(root Node) []Node {
	var nodes []Node
	collectNodesRecursive(root, &nodes)
	return nodes
}

func collectNodesRecursive(node Node, nodes *[]Node) {
	if node == nil {
		return
	}

	*nodes = append(*nodes, node)

	// Visit children based on node type
	switch n := node.(type) {
	case *Module:
		for _, attr := range n.Attributes {
			collectNodesRecursive(&attr, nodes)
		}
		collectNodesRecursive(&n.Name, nodes)
		for _, item := range n.ModuleItems {
			collectNodesRecursive(item, nodes)
		}

	case *Use:
		for _, ns := range n.Namespaces {
			collectNodesRecursive(ns, nodes)
		}
		for _, imp := range n.Imports {
			collectNodesRecursive(imp, nodes)
		}

	case *Namespace:
		collectNodesRecursive(&n.Name, nodes)

	case *ImportItem:
		collectNodesRecursive(&n.Name, nodes)

	case *Struct:
		if n.Attribute != nil {
			collectNodesRecursive(n.Attribute, nodes)
		}
		collectNodesRecursive(&n.Name, nodes)
		for _, item := range n.Items {
			collectNodesRecursive(item, nodes)
		}

	case *StructField:
		collectNodesRecursive(&n.Name, nodes)
		if n.VariableType != nil {
			collectNodesRecursive(n.VariableType, nodes)
		}

	case *VariableType:
		if n.Ref != nil {
			collectNodesRecursive(n.Ref, nodes)
		}
		collectNodesRecursive(&n.Name, nodes)
		for _, generic := range n.Generics {
			collectNodesRecursive(generic, nodes)
		}

	case *RefVariableType:
		if n.Target != nil {
			collectNodesRecursive(n.Target, nodes)
		}

	case *Function:
		if n.Attribute != nil {
			collectNodesRecursive(n.Attribute, nodes)
		}
		collectNodesRecursive(&n.Name, nodes)
		for _, param := range n.Params {
			collectNodesRecursive(param, nodes)
		}
		if n.Return != nil {
			collectNodesRecursive(n.Return, nodes)
		}
		for _, read := range n.Reads {
			collectNodesRecursive(&read, nodes)
		}
		for _, write := range n.Writes {
			collectNodesRecursive(&write, nodes)
		}
		if n.Body != nil {
			collectNodesRecursive(n.Body, nodes)
		}

	case *FunctionParam:
		collectNodesRecursive(&n.Name, nodes)
		if n.Type != nil {
			collectNodesRecursive(n.Type, nodes)
		}

	case *FunctionBlock:
		for _, item := range n.Items {
			collectNodesRecursive(item, nodes)
		}
		if n.TailExpr != nil {
			collectNodesRecursive(n.TailExpr, nodes)
		}

	case *ExprStmt:
		if n.Expr != nil {
			collectNodesRecursive(n.Expr, nodes)
		}

	case *ReturnStmt:
		if n.Value != nil {
			collectNodesRecursive(n.Value, nodes)
		}

	case *LetStmt:
		collectNodesRecursive(&n.Name, nodes)
		if n.Expr != nil {
			collectNodesRecursive(n.Expr, nodes)
		}

	case *AssignStmt:
		if n.Target != nil {
			collectNodesRecursive(n.Target, nodes)
		}
		if n.Value != nil {
			collectNodesRecursive(n.Value, nodes)
		}

	case *AssertStmt:
		for _, arg := range n.Args {
			collectNodesRecursive(arg, nodes)
		}

	case *BinaryExpr:
		if n.Left != nil {
			collectNodesRecursive(n.Left, nodes)
		}
		if n.Right != nil {
			collectNodesRecursive(n.Right, nodes)
		}

	case *UnaryExpr:
		if n.Value != nil {
			collectNodesRecursive(n.Value, nodes)
		}

	case *CallExpr:
		if n.Callee != nil {
			collectNodesRecursive(n.Callee, nodes)
		}
		for _, generic := range n.Generic {
			collectNodesRecursive(&generic, nodes)
		}
		for _, arg := range n.Args {
			collectNodesRecursive(arg, nodes)
		}

	case *FieldAccessExpr:
		if n.Target != nil {
			collectNodesRecursive(n.Target, nodes)
		}

	case *StructLiteralExpr:
		if n.Type != nil {
			collectNodesRecursive(n.Type, nodes)
		}
		for _, field := range n.Fields {
			collectNodesRecursive(&field, nodes)
		}

	case *CalleePath:
		for _, part := range n.Parts {
			collectNodesRecursive(&part, nodes)
		}

	case *StructLiteralField:
		collectNodesRecursive(&n.Name, nodes)
		if n.Value != nil {
			collectNodesRecursive(n.Value, nodes)
		}

	case *ParenExpr:
		if n.Value != nil {
			collectNodesRecursive(n.Value, nodes)
		}
	}
}
