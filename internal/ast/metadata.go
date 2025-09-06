package ast

import "fmt"

// NodeID is a unique identifier for each AST node to track it through compilation
type NodeID uint32

// SourceRange represents a range in the source code
type SourceRange struct {
	Start Position
	End   Position
}

// Metadata contains debugging and compilation information for AST nodes
type Metadata struct {
	// Unique identifier for this AST node
	NodeID NodeID

	// Source location information
	Source SourceRange

	// Original source text for this node (useful for debugging)
	SourceText string

	// Parent node ID (0 if root)
	ParentID NodeID

	// Compilation phase information - will be populated during compilation
	CompilationInfo *CompilationMetadata
}

// CompilationMetadata tracks information through the compilation pipeline
type CompilationMetadata struct {
	// IR node ID (when converted to IR)
	IRID uint32

	// Bytecode address range (start and end addresses)
	BytecodeRange *BytecodeRange

	// Optimization information
	OptimizationInfo *OptimizationInfo

	// Type information resolved during semantic analysis
	TypeInfo *TypeMetadata
}

// BytecodeRange represents the bytecode addresses for this AST node
type BytecodeRange struct {
	StartAddress uint32
	EndAddress   uint32

	// Individual instruction mappings for granular debugging
	Instructions []InstructionMapping
}

// InstructionMapping maps source positions to specific bytecode instructions
type InstructionMapping struct {
	SourcePos   Position
	Address     uint32
	Instruction string // opcode name
	OperandInfo string // operand details
}

// OptimizationInfo tracks how optimizations affected this node
type OptimizationInfo struct {
	// Was this node optimized away?
	OptimizedOut bool

	// What optimization passes affected this node
	OptimizationPasses []string

	// If inlined, what was the original function
	InlinedFrom *NodeID

	// If constant folded, what was the original expression
	ConstantFolded bool
	OriginalValue  string
}

// TypeMetadata contains resolved type information
type TypeMetadata struct {
	// Resolved type name
	TypeName string

	// Generic type parameters
	Generics []string

	// Size in bytes (if known)
	SizeBytes uint32

	// Is this a reference type?
	IsReference bool
	IsMutable   bool
}

// NodeTracker manages node IDs and metadata
type NodeTracker struct {
	nextID   NodeID
	metadata map[NodeID]*Metadata
}

// NewNodeTracker creates a new node tracker
func NewNodeTracker() *NodeTracker {
	return &NodeTracker{
		nextID:   1, // Start at 1, reserve 0 for "no parent"
		metadata: make(map[NodeID]*Metadata),
	}
}

// GenerateID creates a new unique node ID
func (nt *NodeTracker) GenerateID() NodeID {
	id := nt.nextID
	nt.nextID++
	return id
}

// SetMetadata associates metadata with a node ID
func (nt *NodeTracker) SetMetadata(id NodeID, meta *Metadata) {
	nt.metadata[id] = meta
}

// GetMetadata retrieves metadata for a node ID
func (nt *NodeTracker) GetMetadata(id NodeID) *Metadata {
	return nt.metadata[id]
}

// GetAllMetadata returns all metadata (useful for debugging)
func (nt *NodeTracker) GetAllMetadata() map[NodeID]*Metadata {
	return nt.metadata
}

// CreateSourceRange creates a SourceRange from start and end positions
func CreateSourceRange(start, end Position) SourceRange {
	return SourceRange{Start: start, End: end}
}

// Contains checks if a position is within this source range
func (sr SourceRange) Contains(pos Position) bool {
	return sr.Start.Offset <= pos.Offset && pos.Offset <= sr.End.Offset
}

// String returns a human-readable representation of the source range
func (sr SourceRange) String() string {
	if sr.Start.Line == sr.End.Line {
		return fmt.Sprintf("%s:%d:%d-%d", sr.Start.Filename, sr.Start.Line, sr.Start.Column, sr.End.Column)
	}
	return fmt.Sprintf("%s:%d:%d-%d:%d", sr.Start.Filename, sr.Start.Line, sr.Start.Column, sr.End.Line, sr.End.Column)
}

// String returns a human-readable representation of metadata
func (m *Metadata) String() string {
	return fmt.Sprintf("NodeID:%d Source:%s Parent:%d", m.NodeID, m.Source.String(), m.ParentID)
}
