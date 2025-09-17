package ir

import (
	"fmt"
)

// IR types and structures designed for EVM optimization
// This IR uses Static Single Assignment (SSA) form with basic blocks and control flow graphs

// Program represents the entire contract in IR form
type Program struct {
	Contract        string
	Functions       []*Function
	Storage         []*StorageSlot
	Constants       []*Constant
	EventSignatures []*EventSignature
	Blocks          map[string]*BasicBlock
	CFG             *ControlFlowGraph
}

// EventSignature represents a global event signature constant
type EventSignature struct {
	Name      string // e.g. "Transfer_sig"
	EventName string // e.g. "Transfer"
	Signature string // e.g. "Transfer(address,address,uint256)"
}

// Function represents a function in IR form
type Function struct {
	Name       string
	External   bool
	Create     bool
	Params     []*Parameter
	ReturnType Type
	Reads      []string
	Writes     []string
	Entry      *BasicBlock
	Blocks     []*BasicBlock
	LocalVars  map[string]*Value
}

// BasicBlock represents a sequence of instructions with no branches
type BasicBlock struct {
	Label        string
	Instructions []Instruction
	Terminator   Terminator
	Predecessors []*BasicBlock
	Successors   []*BasicBlock
	DominatedBy  *BasicBlock
	Dominates    []*BasicBlock
	LiveIn       map[string]*Value
	LiveOut      map[string]*Value
}

// Value represents a value in SSA form - each value has exactly one definition
type Value struct {
	ID       int
	Name     string
	Type     Type
	DefBlock *BasicBlock
	DefInst  Instruction
	Uses     []*Use
	Version  int // For variable versioning during SSA construction
}

// MemoryRegion represents a region of memory with specific properties
type MemoryRegion struct {
	ID   int
	Name string
	Base *Value // Base pointer/address
	Size *Value // Size in bytes
	Kind MemoryRegionKind
}

// MemoryRegionKind categorizes different types of memory regions
type MemoryRegionKind string

const (
	MemoryRegionABIData    MemoryRegionKind = "abi_data"   // ABI-encoded data
	MemoryRegionScratch    MemoryRegionKind = "scratch"    // Temporary scratch space
	MemoryRegionCalldata   MemoryRegionKind = "calldata"   // Function call data
	MemoryRegionReturnData MemoryRegionKind = "returndata" // Function return data
)

// MemoryEffect represents how an instruction affects memory
type MemoryEffect struct {
	Region *MemoryRegion
	Type   MemoryEffectType
	Offset *Value // Offset within the region (optional)
	Size   *Value // Size of the effect (optional)
}

// MemoryEffectType categorizes memory access patterns
type MemoryEffectType string

const (
	MemoryEffectRead     MemoryEffectType = "read"     // Reads from memory
	MemoryEffectWrite    MemoryEffectType = "write"    // Writes to memory
	MemoryEffectAllocate MemoryEffectType = "allocate" // Allocates memory region
	MemoryEffectFree     MemoryEffectType = "free"     // Frees memory region
)

// Use represents a use of an IR value
type Use struct {
	Value *Value
	User  Instruction
	Block *BasicBlock
}

// Parameter represents a function parameter
type Parameter struct {
	Name  string
	Type  Type
	Value *Value
}

// StorageSlot represents a storage location with optimization metadata
type StorageSlot struct {
	Slot        int
	Name        string
	Type        Type
	AccessCount int            // For optimization - frequently accessed slots
	PackWith    []*StorageSlot // Storage packing optimization
}

// Constant represents a compile-time constant
type Constant struct {
	Value *Value
	Data  interface{}
}

// ControlFlowGraph represents the control flow structure
type ControlFlowGraph struct {
	EntryPoints  []*BasicBlock // All external function entry points
	SuccessExits []*BasicBlock // Blocks ending with RETURN
	FailureExits []*BasicBlock // Blocks ending with REVERT
	Blocks       []*BasicBlock
	Dominance    map[*BasicBlock][]*BasicBlock
	Loops        []*Loop
	Functions    map[string]*FunctionCFG // Per-function CFG information
}

// FunctionCFG represents CFG information for a specific function
type FunctionCFG struct {
	Name         string
	Entry        *BasicBlock
	SuccessExits []*BasicBlock // Function's RETURN blocks
	FailureExits []*BasicBlock // Function's REVERT blocks
	Blocks       []*BasicBlock // All blocks in this function
}

// Loop represents a loop structure for optimization
type Loop struct {
	Header    *BasicBlock
	Body      []*BasicBlock
	Exits     []*BasicBlock
	Invariant []*Value // Loop-invariant values for hoisting
}

// Instructions in SSA form

type Instruction interface {
	GetID() int
	GetResult() *Value
	GetOperands() []*Value
	GetBlock() *BasicBlock
	IsTerminator() bool
	String() string
	GetEffects() []Effect
}

// Effect represents the side effects of an instruction
type Effect interface {
	EffectKind() string
}

// StorageEffect represents effects on contract storage
type StorageEffect struct {
	Type string // "read" or "write"
	Slot int    // Storage slot affected (-1 for dynamic)
}

func (s *StorageEffect) EffectKind() string { return "storage" }

// MemoryEffectOp represents effects on EVM memory
type MemoryEffectOp struct {
	Type   MemoryEffectType
	Region *MemoryRegion
}

func (m *MemoryEffectOp) EffectKind() string { return "memory" }

// PureEffect indicates no side effects
type PureEffect struct{}

func (p *PureEffect) EffectKind() string { return "pure" }

// Terminators end basic blocks
type Terminator interface {
	Instruction
	GetSuccessors() []*BasicBlock
}

// Core SSA Instructions

type PhiInstruction struct {
	ID     int
	Result *Value
	Block  *BasicBlock
	Inputs map[*BasicBlock]*Value
}

type LoadInstruction struct {
	ID      int
	Result  *Value
	Block   *BasicBlock
	Address *Value
}

type StoreInstruction struct {
	ID      int
	Block   *BasicBlock
	Address *Value
	Value   *Value
}

type StorageLoadInstruction struct {
	ID      int
	Result  *Value
	Block   *BasicBlock
	Slot    *Value
	SlotNum int // For optimization - known slot numbers
}

type StorageStoreInstruction struct {
	ID      int
	Block   *BasicBlock
	Slot    *Value
	Value   *Value
	SlotNum int
	Type    Type
}

type KeyedStorageLoadInstruction struct {
	ID       int
	Result   *Value
	Block    *BasicBlock
	Key      *Value
	BaseSlot int
	KeyType  Type
}

type KeyedStorageStoreInstruction struct {
	ID       int
	Block    *BasicBlock
	Key      *Value
	Value    *Value
	BaseSlot int
	KeyType  Type
}

type BinaryInstruction struct {
	ID     int
	Result *Value
	Block  *BasicBlock
	Op     string
	Left   *Value
	Right  *Value
}

type CallInstruction struct {
	ID       int
	Result   *Value
	Block    *BasicBlock
	Function string
	Args     []*Value
	Module   string // For namespace resolution
}

type ConstantInstruction struct {
	ID     int
	Result *Value
	Block  *BasicBlock
	Value  interface{}
	Type   Type
}

// EVM-specific instructions for optimization

type SenderInstruction struct {
	ID     int
	Result *Value
	Block  *BasicBlock
}

type EmitInstruction struct {
	ID    int
	Block *BasicBlock
	Event string
	Args  []*Value
}

type RequireInstruction struct {
	ID        int
	Block     *BasicBlock
	Condition *Value
	Error     *Value
}

// Enhanced IR instructions for optimization

// Abstract storage addressing
type StorageAddrInstruction struct {
	ID       int
	Result   *Value
	Block    *BasicBlock
	BaseSlot int
	Keys     []*Value // 1 key for MAP1, 2 keys for MAP2
}

// Checked arithmetic operations
type CheckedArithInstruction struct {
	ID        int
	ResultVal *Value // The arithmetic result
	ResultOk  *Value // The overflow/underflow check
	Block     *BasicBlock
	Op        string // "ADD_CHK", "SUB_CHK", "MUL_CHK", "DIV_CHK"
	Left      *Value
	Right     *Value
}

// Path assumptions for optimization
type AssumeInstruction struct {
	ID        int
	Block     *BasicBlock
	Predicate *Value
}

// LOG instruction for events
type LogInstruction struct {
	ID        int
	Block     *BasicBlock
	Topics    int // LOG0, LOG1, LOG2, LOG3, LOG4
	Event     string
	Signature *Value   // Event signature hash
	TopicArgs []*Value // Topic arguments (indexed parameters)
	DataPtr   *Value   // Data pointer
	DataLen   *Value   // Data length
}

// Topic address encoding for indexed address parameters
type TopicAddrInstruction struct {
	ID      int
	Result  *Value
	Block   *BasicBlock
	Address *Value
}

// ABI encoding for U256 values in event data
type ABIEncU256Instruction struct {
	ID           int
	ResultData   *Value // Data pointer
	ResultLen    *Value // Data length
	Block        *BasicBlock
	Value        *Value
	MemoryRegion *MemoryRegion  // Memory region allocated for ABI data
	Effects      []MemoryEffect // Memory effects (allocate + write)
}

// Event signature generation instruction (keccak256 hash of event signature)
type EventSignatureInstruction struct {
	ID        int
	Result    *Value
	Block     *BasicBlock
	Event     string // Event name
	Signature string // Full signature like "Transfer(address,address,uint256)"
}

// Revert instruction
type RevertInstruction struct {
	ID    int
	Block *BasicBlock
}

// Terminators

type ReturnTerminator struct {
	ID    int
	Block *BasicBlock
	Value *Value
}

type BranchTerminator struct {
	ID         int
	Block      *BasicBlock
	Condition  *Value
	TrueBlock  *BasicBlock
	FalseBlock *BasicBlock
}

type JumpTerminator struct {
	ID     int
	Block  *BasicBlock
	Target *BasicBlock
}

// Implementation of interfaces

func (p *PhiInstruction) GetID() int        { return p.ID }
func (p *PhiInstruction) GetResult() *Value { return p.Result }
func (p *PhiInstruction) GetOperands() []*Value {
	var ops []*Value
	for _, v := range p.Inputs {
		ops = append(ops, v)
	}
	return ops
}
func (p *PhiInstruction) GetBlock() *BasicBlock { return p.Block }
func (p *PhiInstruction) IsTerminator() bool    { return false }

func (l *LoadInstruction) GetID() int            { return l.ID }
func (l *LoadInstruction) GetResult() *Value     { return l.Result }
func (l *LoadInstruction) GetOperands() []*Value { return []*Value{l.Address} }
func (l *LoadInstruction) GetBlock() *BasicBlock { return l.Block }
func (l *LoadInstruction) IsTerminator() bool    { return false }

func (s *StoreInstruction) GetID() int            { return s.ID }
func (s *StoreInstruction) GetResult() *Value     { return nil }
func (s *StoreInstruction) GetOperands() []*Value { return []*Value{s.Address, s.Value} }
func (s *StoreInstruction) GetBlock() *BasicBlock { return s.Block }
func (s *StoreInstruction) IsTerminator() bool    { return false }

func (s *StorageLoadInstruction) GetID() int            { return s.ID }
func (s *StorageLoadInstruction) GetResult() *Value     { return s.Result }
func (s *StorageLoadInstruction) GetOperands() []*Value { return []*Value{s.Slot} }
func (s *StorageLoadInstruction) GetBlock() *BasicBlock { return s.Block }
func (s *StorageLoadInstruction) IsTerminator() bool    { return false }

func (s *StorageStoreInstruction) GetID() int            { return s.ID }
func (s *StorageStoreInstruction) GetResult() *Value     { return nil }
func (s *StorageStoreInstruction) GetOperands() []*Value { return []*Value{s.Slot, s.Value} }
func (s *StorageStoreInstruction) GetBlock() *BasicBlock { return s.Block }
func (s *StorageStoreInstruction) IsTerminator() bool    { return false }

func (k *KeyedStorageLoadInstruction) GetID() int            { return k.ID }
func (k *KeyedStorageLoadInstruction) GetResult() *Value     { return k.Result }
func (k *KeyedStorageLoadInstruction) GetOperands() []*Value { return []*Value{k.Key} }
func (k *KeyedStorageLoadInstruction) GetBlock() *BasicBlock { return k.Block }
func (k *KeyedStorageLoadInstruction) IsTerminator() bool    { return false }

func (k *KeyedStorageStoreInstruction) GetID() int            { return k.ID }
func (k *KeyedStorageStoreInstruction) GetResult() *Value     { return nil }
func (k *KeyedStorageStoreInstruction) GetOperands() []*Value { return []*Value{k.Key, k.Value} }
func (k *KeyedStorageStoreInstruction) GetBlock() *BasicBlock { return k.Block }
func (k *KeyedStorageStoreInstruction) IsTerminator() bool    { return false }

func (b *BinaryInstruction) GetID() int            { return b.ID }
func (b *BinaryInstruction) GetResult() *Value     { return b.Result }
func (b *BinaryInstruction) GetOperands() []*Value { return []*Value{b.Left, b.Right} }
func (b *BinaryInstruction) GetBlock() *BasicBlock { return b.Block }
func (b *BinaryInstruction) IsTerminator() bool    { return false }

func (c *CallInstruction) GetID() int            { return c.ID }
func (c *CallInstruction) GetResult() *Value     { return c.Result }
func (c *CallInstruction) GetOperands() []*Value { return c.Args }
func (c *CallInstruction) GetBlock() *BasicBlock { return c.Block }
func (c *CallInstruction) IsTerminator() bool    { return false }

func (c *ConstantInstruction) GetID() int            { return c.ID }
func (c *ConstantInstruction) GetResult() *Value     { return c.Result }
func (c *ConstantInstruction) GetOperands() []*Value { return []*Value{} }
func (c *ConstantInstruction) GetBlock() *BasicBlock { return c.Block }
func (c *ConstantInstruction) IsTerminator() bool    { return false }

func (s *SenderInstruction) GetID() int            { return s.ID }
func (s *SenderInstruction) GetResult() *Value     { return s.Result }
func (s *SenderInstruction) GetOperands() []*Value { return []*Value{} }
func (s *SenderInstruction) GetBlock() *BasicBlock { return s.Block }
func (s *SenderInstruction) IsTerminator() bool    { return false }

func (e *EmitInstruction) GetID() int            { return e.ID }
func (e *EmitInstruction) GetResult() *Value     { return nil }
func (e *EmitInstruction) GetOperands() []*Value { return e.Args }
func (e *EmitInstruction) GetBlock() *BasicBlock { return e.Block }
func (e *EmitInstruction) IsTerminator() bool    { return false }

func (r *RequireInstruction) GetID() int            { return r.ID }
func (r *RequireInstruction) GetResult() *Value     { return nil }
func (r *RequireInstruction) GetOperands() []*Value { return []*Value{r.Condition, r.Error} }
func (r *RequireInstruction) GetBlock() *BasicBlock { return r.Block }
func (r *RequireInstruction) IsTerminator() bool    { return false }

// Terminator implementations

func (r *ReturnTerminator) GetID() int        { return r.ID }
func (r *ReturnTerminator) GetResult() *Value { return nil }
func (r *ReturnTerminator) GetOperands() []*Value {
	if r.Value != nil {
		return []*Value{r.Value}
	}
	return []*Value{}
}
func (r *ReturnTerminator) GetBlock() *BasicBlock        { return r.Block }
func (r *ReturnTerminator) IsTerminator() bool           { return true }
func (r *ReturnTerminator) GetSuccessors() []*BasicBlock { return []*BasicBlock{} }

func (b *BranchTerminator) GetID() int            { return b.ID }
func (b *BranchTerminator) GetResult() *Value     { return nil }
func (b *BranchTerminator) GetOperands() []*Value { return []*Value{b.Condition} }
func (b *BranchTerminator) GetBlock() *BasicBlock { return b.Block }
func (b *BranchTerminator) IsTerminator() bool    { return true }
func (b *BranchTerminator) GetSuccessors() []*BasicBlock {
	return []*BasicBlock{b.TrueBlock, b.FalseBlock}
}

func (j *JumpTerminator) GetID() int                   { return j.ID }
func (j *JumpTerminator) GetResult() *Value            { return nil }
func (j *JumpTerminator) GetOperands() []*Value        { return []*Value{} }
func (j *JumpTerminator) GetBlock() *BasicBlock        { return j.Block }
func (j *JumpTerminator) IsTerminator() bool           { return true }
func (j *JumpTerminator) GetSuccessors() []*BasicBlock { return []*BasicBlock{j.Target} }

// Enhanced instruction implementations

func (s *StorageAddrInstruction) GetID() int            { return s.ID }
func (s *StorageAddrInstruction) GetResult() *Value     { return s.Result }
func (s *StorageAddrInstruction) GetOperands() []*Value { return s.Keys }
func (s *StorageAddrInstruction) GetBlock() *BasicBlock { return s.Block }
func (s *StorageAddrInstruction) IsTerminator() bool    { return false }

func (c *CheckedArithInstruction) GetID() int            { return c.ID }
func (c *CheckedArithInstruction) GetResult() *Value     { return c.ResultVal } // Primary result
func (c *CheckedArithInstruction) GetOperands() []*Value { return []*Value{c.Left, c.Right} }
func (c *CheckedArithInstruction) GetBlock() *BasicBlock { return c.Block }
func (c *CheckedArithInstruction) IsTerminator() bool    { return false }

func (a *AssumeInstruction) GetID() int            { return a.ID }
func (a *AssumeInstruction) GetResult() *Value     { return nil }
func (a *AssumeInstruction) GetOperands() []*Value { return []*Value{a.Predicate} }
func (a *AssumeInstruction) GetBlock() *BasicBlock { return a.Block }
func (a *AssumeInstruction) IsTerminator() bool    { return false }

func (l *LogInstruction) GetID() int        { return l.ID }
func (l *LogInstruction) GetResult() *Value { return nil }
func (l *LogInstruction) GetOperands() []*Value {
	operands := []*Value{l.Signature}
	operands = append(operands, l.TopicArgs...)
	if l.DataPtr != nil {
		operands = append(operands, l.DataPtr)
	}
	if l.DataLen != nil {
		operands = append(operands, l.DataLen)
	}
	return operands
}
func (l *LogInstruction) GetBlock() *BasicBlock { return l.Block }
func (l *LogInstruction) IsTerminator() bool    { return false }

func (t *TopicAddrInstruction) GetID() int            { return t.ID }
func (t *TopicAddrInstruction) GetResult() *Value     { return t.Result }
func (t *TopicAddrInstruction) GetOperands() []*Value { return []*Value{t.Address} }
func (t *TopicAddrInstruction) GetBlock() *BasicBlock { return t.Block }
func (t *TopicAddrInstruction) IsTerminator() bool    { return false }

func (a *ABIEncU256Instruction) GetID() int            { return a.ID }
func (a *ABIEncU256Instruction) GetResult() *Value     { return a.ResultData } // Return data pointer as primary result
func (a *ABIEncU256Instruction) GetOperands() []*Value { return []*Value{a.Value} }
func (a *ABIEncU256Instruction) GetBlock() *BasicBlock { return a.Block }
func (a *ABIEncU256Instruction) IsTerminator() bool    { return false }

func (e *EventSignatureInstruction) GetID() int            { return e.ID }
func (e *EventSignatureInstruction) GetResult() *Value     { return e.Result }
func (e *EventSignatureInstruction) GetOperands() []*Value { return []*Value{} }
func (e *EventSignatureInstruction) GetBlock() *BasicBlock { return e.Block }
func (e *EventSignatureInstruction) IsTerminator() bool    { return false }

func (r *RevertInstruction) GetID() int            { return r.ID }
func (r *RevertInstruction) GetResult() *Value     { return nil }
func (r *RevertInstruction) GetOperands() []*Value { return []*Value{} }
func (r *RevertInstruction) GetBlock() *BasicBlock { return r.Block }
func (r *RevertInstruction) IsTerminator() bool    { return true }

// RevertInstruction implements Terminator interface
func (r *RevertInstruction) GetSuccessors() []*BasicBlock { return []*BasicBlock{} }

// Types (reusing from original IR)

type Type interface {
	String() string
}

type IntType struct {
	Bits int
}

type BoolType struct{}

type AddressType struct{}

type StringType struct{}

type SlotsType struct {
	KeyType   Type
	ValueType Type
}

type TupleType struct {
	Elements []Type
}

type StorageAddrType struct {
	// Abstract storage address type for SADDR_MAP1/MAP2 results
}

func (i *IntType) String() string          { return fmt.Sprintf("U%d", i.Bits) }
func (b *BoolType) String() string         { return "Bool" }
func (a *AddressType) String() string      { return "Address" }
func (s *StringType) String() string       { return "String" }
func (s *SlotsType) String() string        { return fmt.Sprintf("Slots<%s, %s>", s.KeyType, s.ValueType) }
func (sa *StorageAddrType) String() string { return "StorageAddr" }
func (t *TupleType) String() string {
	if len(t.Elements) == 0 {
		return "()"
	}
	result := "("
	for i, elem := range t.Elements {
		if i > 0 {
			result += ", "
		}
		result += elem.String()
	}
	return result + ")"
}
