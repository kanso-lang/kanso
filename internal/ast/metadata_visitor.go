package ast

import "strings"

// MetadataVisitor provides utilities for working with metadata across the AST
type MetadataVisitor struct {
	tracker    *NodeTracker
	sourceText string
}

// NewMetadataVisitor creates a new metadata visitor
func NewMetadataVisitor(sourceText string) *MetadataVisitor {
	return &MetadataVisitor{
		tracker:    NewNodeTracker(),
		sourceText: sourceText,
	}
}

// AssignMetadata assigns metadata to a node and all its children
func (mv *MetadataVisitor) AssignMetadata(node Node, parentID NodeID) {
	if node == nil {
		return
	}

	// Generate unique ID for this node
	nodeID := mv.tracker.GenerateID()

	// Extract source text for this node
	start := node.NodePos()
	end := node.NodeEndPos()
	sourceText := mv.extractSourceText(start, end)

	// Create metadata
	metadata := &Metadata{
		NodeID:     nodeID,
		Source:     CreateSourceRange(start, end),
		SourceText: sourceText,
		ParentID:   parentID,
	}

	// Assign to node
	node.SetMetadata(metadata)
	mv.tracker.SetMetadata(nodeID, metadata)

	// Visit children recursively
	mv.visitChildren(node, nodeID)
}

// extractSourceText extracts the source text between two positions
func (mv *MetadataVisitor) extractSourceText(start, end Position) string {
	if mv.sourceText == "" {
		return ""
	}

	if start.Offset < 0 || end.Offset < 0 || start.Offset > len(mv.sourceText) || end.Offset > len(mv.sourceText) {
		return ""
	}

	if start.Offset > end.Offset {
		return ""
	}

	return mv.sourceText[start.Offset:end.Offset]
}

// visitChildren visits all children of a node
func (mv *MetadataVisitor) visitChildren(node Node, parentID NodeID) {
	switch n := node.(type) {
	case *Module:
		for _, attr := range n.Attributes {
			mv.AssignMetadata(&attr, parentID)
		}
		mv.AssignMetadata(&n.Name, parentID)
		for _, item := range n.ModuleItems {
			mv.AssignMetadata(item, parentID)
		}

	case *Use:
		for _, ns := range n.Namespaces {
			mv.AssignMetadata(ns, parentID)
		}
		for _, imp := range n.Imports {
			mv.AssignMetadata(imp, parentID)
		}

	case *Namespace:
		mv.AssignMetadata(&n.Name, parentID)

	case *ImportItem:
		mv.AssignMetadata(&n.Name, parentID)

	case *Struct:
		if n.Attribute != nil {
			mv.AssignMetadata(n.Attribute, parentID)
		}
		mv.AssignMetadata(&n.Name, parentID)
		for _, item := range n.Items {
			mv.AssignMetadata(item, parentID)
		}

	case *StructField:
		mv.AssignMetadata(&n.Name, parentID)
		if n.VariableType != nil {
			mv.AssignMetadata(n.VariableType, parentID)
		}

	case *VariableType:
		if n.Ref != nil {
			mv.AssignMetadata(n.Ref, parentID)
		}
		mv.AssignMetadata(&n.Name, parentID)
		for _, generic := range n.Generics {
			mv.AssignMetadata(generic, parentID)
		}

	case *RefVariableType:
		if n.Target != nil {
			mv.AssignMetadata(n.Target, parentID)
		}

	case *Function:
		if n.Attribute != nil {
			mv.AssignMetadata(n.Attribute, parentID)
		}
		mv.AssignMetadata(&n.Name, parentID)
		for _, param := range n.Params {
			mv.AssignMetadata(param, parentID)
		}
		if n.Return != nil {
			mv.AssignMetadata(n.Return, parentID)
		}
		for _, read := range n.Reads {
			mv.AssignMetadata(&read, parentID)
		}
		for _, write := range n.Writes {
			mv.AssignMetadata(&write, parentID)
		}
		if n.Body != nil {
			mv.AssignMetadata(n.Body, parentID)
		}

	case *FunctionParam:
		mv.AssignMetadata(&n.Name, parentID)
		if n.Type != nil {
			mv.AssignMetadata(n.Type, parentID)
		}

	case *FunctionBlock:
		for _, item := range n.Items {
			mv.AssignMetadata(item, parentID)
		}
		if n.TailExpr != nil {
			mv.AssignMetadata(n.TailExpr, parentID)
		}

	case *ExprStmt:
		if n.Expr != nil {
			mv.AssignMetadata(n.Expr, parentID)
		}

	case *ReturnStmt:
		if n.Value != nil {
			mv.AssignMetadata(n.Value, parentID)
		}

	case *LetStmt:
		mv.AssignMetadata(&n.Name, parentID)
		if n.Expr != nil {
			mv.AssignMetadata(n.Expr, parentID)
		}

	case *AssignStmt:
		if n.Target != nil {
			mv.AssignMetadata(n.Target, parentID)
		}
		if n.Value != nil {
			mv.AssignMetadata(n.Value, parentID)
		}

	case *AssertStmt:
		for _, arg := range n.Args {
			mv.AssignMetadata(arg, parentID)
		}

	case *BinaryExpr:
		if n.Left != nil {
			mv.AssignMetadata(n.Left, parentID)
		}
		if n.Right != nil {
			mv.AssignMetadata(n.Right, parentID)
		}

	case *UnaryExpr:
		if n.Value != nil {
			mv.AssignMetadata(n.Value, parentID)
		}

	case *CallExpr:
		if n.Callee != nil {
			mv.AssignMetadata(n.Callee, parentID)
		}
		for _, generic := range n.Generic {
			mv.AssignMetadata(&generic, parentID)
		}
		for _, arg := range n.Args {
			mv.AssignMetadata(arg, parentID)
		}

	case *FieldAccessExpr:
		if n.Target != nil {
			mv.AssignMetadata(n.Target, parentID)
		}

	case *StructLiteralExpr:
		if n.Type != nil {
			mv.AssignMetadata(n.Type, parentID)
		}
		for _, field := range n.Fields {
			mv.AssignMetadata(&field, parentID)
		}

	case *CalleePath:
		for _, part := range n.Parts {
			mv.AssignMetadata(&part, parentID)
		}

	case *StructLiteralField:
		mv.AssignMetadata(&n.Name, parentID)
		if n.Value != nil {
			mv.AssignMetadata(n.Value, parentID)
		}

	case *ParenExpr:
		if n.Value != nil {
			mv.AssignMetadata(n.Value, parentID)
		}
	}
}

// GetTracker returns the node tracker
func (mv *MetadataVisitor) GetTracker() *NodeTracker {
	return mv.tracker
}

// FindNodeByPosition finds a node at a specific position
func (mv *MetadataVisitor) FindNodeByPosition(pos Position) *Metadata {
	for _, meta := range mv.tracker.metadata {
		if meta.Source.Contains(pos) {
			return meta
		}
	}
	return nil
}

// GetNodesByType returns all nodes of a specific type
func (mv *MetadataVisitor) GetNodesByType(nodeType NodeType) []*Metadata {
	var result []*Metadata
	for nodeID := range mv.tracker.metadata {
		// We would need the actual node to check its type
		// For now, this is a placeholder
		_ = nodeID
	}
	return result
}

// PrintDebugInfo prints debugging information about all nodes
func (mv *MetadataVisitor) PrintDebugInfo() string {
	var sb strings.Builder
	sb.WriteString("=== AST Metadata Debug Info ===\n")

	for nodeID, meta := range mv.tracker.metadata {
		sb.WriteString(string(rune('A' + int(nodeID-1))))
		sb.WriteString(": ")
		sb.WriteString(meta.String())
		sb.WriteString("\n")

		if meta.SourceText != "" {
			sb.WriteString("   Source: ")
			sb.WriteString(strings.ReplaceAll(meta.SourceText, "\n", "\\n"))
			sb.WriteString("\n")
		}

		if meta.CompilationInfo != nil {
			sb.WriteString("   Compilation: ")
			if meta.CompilationInfo.IRID != 0 {
				sb.WriteString("IR:")
				sb.WriteString(string(rune('0' + int(meta.CompilationInfo.IRID))))
				sb.WriteString(" ")
			}
			if meta.CompilationInfo.BytecodeRange != nil {
				sb.WriteString("Bytecode:")
				sb.WriteString(string(rune('0' + int(meta.CompilationInfo.BytecodeRange.StartAddress))))
				sb.WriteString("-")
				sb.WriteString(string(rune('0' + int(meta.CompilationInfo.BytecodeRange.EndAddress))))
			}
			sb.WriteString("\n")
		}
		sb.WriteString("\n")
	}

	return sb.String()
}
