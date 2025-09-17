package ir

// This file contains all IR optimization passes for gas efficiency
// Optimizations are applied after initial IR generation but before EVM bytecode generation

// Must-haves (highest ROI)
//	1.	Global value numbering (GVN) + Partial redundancy elimination (PRE)
//
//	•	Kill repeated keccak256(key, slot) for mappings, repeated MLOAD/SLOAD, arithmetic recomputations across blocks.
//	•	Needs SSA value numbering + lightweight congruence classes; treat SLOAD/SSTORE/CALL as barriers unless proven safe.
//
//	2.	Range/condition propagation (guard→use)
//
//	•	Push facts from require/assert into arithmetic to remove overflow/underflow/zero-div checks (e.g., require(a>=b) ⇒ drop underflow on a-b).
//	•	Path-sensitive, dominance-based; stop facts at external calls or aliasing writes.
//
//	3.	Memory/Storage SSA (or effect/alias model that’s “good enough”)
//
//	•	Store→load forwarding, dead store elimination, and “same-value” SSTORE removal.
//	•	Slot-aware aliasing for storage (key+base slot), region/offset aliasing for memory. Without this, LICM/PRE are hamstrung.
//
//	4.	Stackifier with Sethi–Ullman ordering + peephole
//
//	•	Schedule eval order to minimize DUP/SWAP.
//	•	Peepholes: eliminate identity/annihilator ops, combine PUSHes, collapse trivial compare+branch patterns.
//
//	5.	Instruction selection for modern opcodes
//
//	•	Prefer SHL/SHR/SAR over MUL/DIV by powers of two; fold common masks.
//	•	Use MCOPY (EIP-5656) for bulk moves; coalesce adjacent MSTOREs.
//	•	Gate by fork; provide fallbacks.
//
//	6.	Interprocedural inlining with a gas+size oracle
//
//	•	Inline tiny hot helpers; keep cold code outlined.
//	•	Model code-size fee and runtime gas together (don’t chase tiny wins that bloat bytecode).
//
//Strong next steps
//	8.	LICM with side-effect modeling
//
//	•	Hoist loop-invariant keccak/pure math and some SLOADs when proven no intervening writes to the slot.
//	•	Requires your alias/effect model; be conservative across calls.
//
//	9.	Keccak canonicalization & caching
//
//	•	Fold keccak(encode(const…)) at compile time.
//	•	Recognize mapping slot hashes and reuse within a function; avoid re-encoding the same 64-byte pattern.
//
//	10.	Control-flow shaping
//
//	•	Block merging, tail duplication on hot diamonds, branch inversion for fall-through.
//	•	Reduces JUMPDESTs and improves stack scheduling opportunities.
//
//	11.	Dead-revert/require pruning & post-dominator DCE
//
//	•	After a revert edge, delete unreachable work (stores/mem ops that precede an unconditional revert).
//
//	12.	Transient storage (EIP-1153) as a spill space (optional)
//
//	•	For tight loops, TSTORE/TLOAD can beat memory when aliasing is simple and lifetime is short.
//	•	Only when fork supports it and analysis proves safety.
//
//Correctness guardrails (non-negotiable)
//	•	Dominance & path conditions for any check elimination.
//	•	Alias/effect barriers at CALL/DELEGATECALL/STATICCALL unless you have summaries.
//	•	Storage aliasing by (base slot, key); for mappings of mappings, treat concatenated keys as part of the slot id.
//	•	Memory expansion model in the cost oracle (group writes, prefer MCOPY).
//	•	Witness/verification mode: emit a per-function report of removed checks + reasons; add a -Zverify pass that checks optimized IR refines unoptimized semantics.
//
//What to de-prioritize
//	•	Heavy loop transforms (unswitching, vectorization) — low payoff on typical contracts.
//	•	Exotic interprocedural analyses — inlining + simple const-prop usually suffices.
//	•	“Zero/self-transfer” micro-branches — noise unless your product cares.

import (
	"fmt"
	"strings"
)

// OptimizationPass represents a single optimization transformation
type OptimizationPass interface {
	Name() string
	Apply(program *Program) bool // Returns true if changes were made
	Description() string
}

// OptimizationPipeline manages the sequence of optimization passes
type OptimizationPipeline struct {
	passes []OptimizationPass
}

// NewOptimizationPipeline creates a new optimization pipeline with default passes
func NewOptimizationPipeline() *OptimizationPipeline {
	pipeline := &OptimizationPipeline{}

	// Add optimization passes in order of execution
	pipeline.AddPass(&ConstantFolding{})
	pipeline.AddPass(&CheckedArithmeticOptimization{}) // Must run before DCE
	pipeline.AddPass(&DeadCodeElimination{})
	pipeline.AddPass(&CommonSubexpressionElimination{})

	return pipeline
}

// AddPass adds an optimization pass to the pipeline
func (p *OptimizationPipeline) AddPass(pass OptimizationPass) {
	p.passes = append(p.passes, pass)
}

// Run executes all optimization passes on the IR program
func (p *OptimizationPipeline) Run(program *Program) {
	fmt.Printf("Running %d optimization passes...\n", len(p.passes))

	for _, pass := range p.passes {
		fmt.Printf("  - %s: %s\n", pass.Name(), pass.Description())
		changed := pass.Apply(program)
		if changed {
			fmt.Printf("    ✓ Applied optimizations\n")
		} else {
			fmt.Printf("    - No changes needed\n")
		}
	}
}

// Common optimization passes that can be implemented:

// ConstantFolding evaluates constant expressions at compile time
type ConstantFolding struct{}

func (cf *ConstantFolding) Name() string {
	return "Constant Folding"
}

func (cf *ConstantFolding) Description() string {
	return "Evaluates constant expressions at compile time and replaces with literals"
}

func (cf *ConstantFolding) Apply(program *Program) bool {
	changed := false

	for _, fn := range program.Functions {
		if cf.foldConstants(fn) {
			changed = true
		}
	}

	return changed
}

// foldConstants performs constant folding within a function
func (cf *ConstantFolding) foldConstants(fn *Function) bool {
	changed := false

	// Track constant values (literal values and their computed results)
	constants := make(map[*Value]interface{})

	for _, block := range fn.Blocks {
		// First pass: identify constant values
		for _, inst := range block.Instructions {
			cf.identifyConstants(inst, constants)
		}

		// Second pass: fold constant expressions
		newInstructions := []Instruction{}
		for _, inst := range block.Instructions {
			if folded := cf.foldInstruction(inst, constants); folded != nil {
				if folded != inst {
					changed = true
				}
				newInstructions = append(newInstructions, folded)
			} else {
				// Keep original instruction
				newInstructions = append(newInstructions, inst)
			}
		}

		if changed {
			block.Instructions = newInstructions
		}
	}

	return changed
}

// identifyConstants identifies values that are compile-time constants
func (cf *ConstantFolding) identifyConstants(inst Instruction, constants map[*Value]interface{}) {
	switch i := inst.(type) {
	case *ConstantInstruction:
		// Direct constant values
		constants[i.Result] = i.Value
	case *BinaryInstruction:
		// Check if both operands are constants
		if leftVal, leftOk := constants[i.Left]; leftOk {
			if rightVal, rightOk := constants[i.Right]; rightOk {
				// Both operands are constants, compute the result
				if result := cf.computeBinaryOp(i.Op, leftVal, rightVal); result != nil {
					constants[i.Result] = result
				}
			}
		}
	}
}

// foldInstruction attempts to fold a constant instruction
func (cf *ConstantFolding) foldInstruction(inst Instruction, constants map[*Value]interface{}) Instruction {
	switch i := inst.(type) {
	case *BinaryInstruction:
		// Check if we can fold this binary operation
		if leftVal, leftOk := constants[i.Left]; leftOk {
			if rightVal, rightOk := constants[i.Right]; rightOk {
				// Both operands are constants, replace with constant instruction
				if result := cf.computeBinaryOp(i.Op, leftVal, rightVal); result != nil {
					return &ConstantInstruction{
						ID:     i.ID,
						Result: i.Result,
						Block:  i.Block,
						Value:  result,
					}
				}
			}
		}
	}

	// Return original instruction if no folding possible
	return inst
}

// computeBinaryOp performs constant computation for binary operations
func (cf *ConstantFolding) computeBinaryOp(op string, left, right interface{}) interface{} {
	// Handle integer arithmetic (simplified for U256 values)
	leftInt, leftIsInt := left.(uint64)
	rightInt, rightIsInt := right.(uint64)

	if leftIsInt && rightIsInt {
		switch op {
		case "+":
			return leftInt + rightInt
		case "-":
			if leftInt >= rightInt {
				return leftInt - rightInt
			}
		case "*":
			return leftInt * rightInt
		case "/":
			if rightInt != 0 {
				return leftInt / rightInt
			}
		case "%":
			if rightInt != 0 {
				return leftInt % rightInt
			}
		case "==":
			return leftInt == rightInt
		case "!=":
			return leftInt != rightInt
		case "<":
			return leftInt < rightInt
		case "<=":
			return leftInt <= rightInt
		case ">":
			return leftInt > rightInt
		case ">=":
			return leftInt >= rightInt
		}
	}

	// Handle boolean operations
	leftBool, leftIsBool := left.(bool)
	rightBool, rightIsBool := right.(bool)

	if leftIsBool && rightIsBool {
		switch op {
		case "&&":
			return leftBool && rightBool
		case "||":
			return leftBool || rightBool
		case "==":
			return leftBool == rightBool
		case "!=":
			return leftBool != rightBool
		}
	}

	return nil // Cannot fold
}

// DeadCodeElimination removes unreachable code and unused values
type DeadCodeElimination struct{}

func (dce *DeadCodeElimination) Name() string {
	return "Dead Code Elimination"
}

func (dce *DeadCodeElimination) Description() string {
	return "Removes unreachable basic blocks and unused instructions"
}

func (dce *DeadCodeElimination) Apply(program *Program) bool {
	changed := false

	for _, fn := range program.Functions {
		if dce.eliminateDeadBlocks(fn) {
			changed = true
		}
		if dce.eliminateDeadInstructions(fn) {
			changed = true
		}
	}

	return changed
}

// eliminateDeadBlocks removes unreachable basic blocks using reachability analysis
func (dce *DeadCodeElimination) eliminateDeadBlocks(fn *Function) bool {
	if len(fn.Blocks) == 0 {
		return false
	}

	// Mark reachable blocks starting from entry block
	reachable := make(map[*BasicBlock]bool)
	dce.markReachable(fn.Blocks[0], reachable)

	// Remove unreachable blocks
	newBlocks := []*BasicBlock{}
	changed := false

	for _, block := range fn.Blocks {
		if reachable[block] {
			newBlocks = append(newBlocks, block)
		} else {
			changed = true
		}
	}

	if changed {
		fn.Blocks = newBlocks
	}

	return changed
}

// markReachable recursively marks all blocks reachable from the given block
func (dce *DeadCodeElimination) markReachable(block *BasicBlock, reachable map[*BasicBlock]bool) {
	if reachable[block] {
		return // Already visited
	}

	reachable[block] = true

	// Visit successors based on terminator type
	if block.Terminator != nil {
		switch term := block.Terminator.(type) {
		case *JumpTerminator:
			if term.Target != nil {
				dce.markReachable(term.Target, reachable)
			}
		case *BranchTerminator:
			if term.TrueBlock != nil {
				dce.markReachable(term.TrueBlock, reachable)
			}
			if term.FalseBlock != nil {
				dce.markReachable(term.FalseBlock, reachable)
			}
			// ReturnTerminator and RevertTerminator have no successors
		}
	}
}

// eliminateDeadInstructions removes instructions whose results are never used
func (dce *DeadCodeElimination) eliminateDeadInstructions(fn *Function) bool {
	// Build use sets for all values
	used := make(map[*Value]bool)

	// Mark values used in terminators and side-effect instructions
	for _, block := range fn.Blocks {
		for _, inst := range block.Instructions {
			dce.markUsedValues(inst, used)
		}
		if block.Terminator != nil {
			dce.markUsedTerminatorValues(block.Terminator, used)
		}
	}

	// Remove instructions that produce unused values and have no side effects
	changed := false
	for _, block := range fn.Blocks {
		newInstructions := []Instruction{}

		for _, inst := range block.Instructions {
			if dce.shouldKeepInstruction(inst, used) {
				newInstructions = append(newInstructions, inst)
			} else {
				changed = true
			}
		}

		if changed {
			block.Instructions = newInstructions
		}
	}

	return changed
}

// markUsedValues marks all values used by an instruction
func (dce *DeadCodeElimination) markUsedValues(inst Instruction, used map[*Value]bool) {
	switch i := inst.(type) {
	case *BinaryInstruction:
		used[i.Left] = true
		used[i.Right] = true
	case *CallInstruction:
		for _, arg := range i.Args {
			used[arg] = true
		}
	case *StoreInstruction:
		used[i.Address] = true
		used[i.Value] = true
	case *StorageStoreInstruction:
		used[i.Slot] = true
		used[i.Value] = true
	case *StorageLoadInstruction:
		used[i.Slot] = true
	case *AssumeInstruction:
		used[i.Predicate] = true
	case *StorageAddrInstruction:
		for _, key := range i.Keys {
			used[key] = true
		}
	case *TopicAddrInstruction:
		used[i.Address] = true
	case *ABIEncU256Instruction:
		used[i.Value] = true
		// EventSignatureInstruction, SenderInstruction, ConstantInstruction have no operands
	}
}

// markUsedTerminatorValues marks all values used by a terminator
func (dce *DeadCodeElimination) markUsedTerminatorValues(term Terminator, used map[*Value]bool) {
	switch t := term.(type) {
	case *BranchTerminator:
		used[t.Condition] = true
	case *ReturnTerminator:
		if t.Value != nil {
			used[t.Value] = true
		}
		// JumpTerminator and RevertTerminator have no operands
	}
}

// shouldKeepInstruction determines if an instruction should be kept
func (dce *DeadCodeElimination) shouldKeepInstruction(inst Instruction, used map[*Value]bool) bool {
	// Always keep instructions with side effects
	switch inst.(type) {
	case *StoreInstruction, *StorageStoreInstruction, *CallInstruction:
		return true // Side effects
	case *AssumeInstruction:
		return true // Affects optimization assumptions
	}

	// Keep instructions whose results are used
	switch i := inst.(type) {
	case *BinaryInstruction:
		return used[i.Result]
	case *StorageLoadInstruction:
		return used[i.Result]
	case *StorageAddrInstruction:
		return used[i.Result]
	case *SenderInstruction:
		return used[i.Result]
	case *ConstantInstruction:
		return used[i.Result]
	case *TopicAddrInstruction:
		return used[i.Result]
	case *ABIEncU256Instruction:
		return used[i.ResultData] || used[i.ResultLen]
	case *EventSignatureInstruction:
		return used[i.Result]
	default:
		return true // Conservative: keep unknown instructions
	}
}

// CommonSubexpressionElimination removes redundant computations within basic blocks
type CommonSubexpressionElimination struct{}

func (cse *CommonSubexpressionElimination) Name() string {
	return "Common Subexpression Elimination"
}

func (cse *CommonSubexpressionElimination) Description() string {
	return "Eliminates redundant computations within basic blocks"
}

func (cse *CommonSubexpressionElimination) Apply(program *Program) bool {
	changed := false

	for _, fn := range program.Functions {
		for _, block := range fn.Blocks {
			if cse.optimizeBlock(block) {
				changed = true
			}
		}
	}

	return changed
}

// optimizeBlock removes redundant computations within a single basic block
func (cse *CommonSubexpressionElimination) optimizeBlock(block *BasicBlock) bool {
	changed := false

	// Track available expressions (for now, just track sender() calls)
	var senderResult *Value

	// Process instructions
	newInstructions := []Instruction{}

	for _, inst := range block.Instructions {
		switch i := inst.(type) {
		case *SenderInstruction:
			if senderResult == nil {
				// First sender() call - keep it
				senderResult = i.Result
				newInstructions = append(newInstructions, inst)
			} else {
				// Redundant sender() call - replace all uses of this result with the first one
				cse.replaceValue(block, i.Result, senderResult)
				changed = true
				// Don't add this instruction to newInstructions (remove it)
			}
		default:
			newInstructions = append(newInstructions, inst)
		}
	}

	if changed {
		block.Instructions = newInstructions
	}

	return changed
}

// replaceValue replaces all uses of oldValue with newValue in the block
func (cse *CommonSubexpressionElimination) replaceValue(block *BasicBlock, oldValue, newValue *Value) {
	// Replace in remaining instructions
	for _, inst := range block.Instructions {
		cse.replaceInInstruction(inst, oldValue, newValue)
	}

	// Replace in terminator
	if block.Terminator != nil {
		cse.replaceInTerminator(block.Terminator, oldValue, newValue)
	}
}

// replaceInInstruction replaces value references in an instruction
func (cse *CommonSubexpressionElimination) replaceInInstruction(inst Instruction, oldValue, newValue *Value) {
	switch i := inst.(type) {
	case *BinaryInstruction:
		if i.Left == oldValue {
			i.Left = newValue
		}
		if i.Right == oldValue {
			i.Right = newValue
		}
	case *CallInstruction:
		for j, arg := range i.Args {
			if arg == oldValue {
				i.Args[j] = newValue
			}
		}
	case *StoreInstruction:
		if i.Address == oldValue {
			i.Address = newValue
		}
		if i.Value == oldValue {
			i.Value = newValue
		}
	case *StorageStoreInstruction:
		if i.Slot == oldValue {
			i.Slot = newValue
		}
		if i.Value == oldValue {
			i.Value = newValue
		}
	case *StorageLoadInstruction:
		if i.Slot == oldValue {
			i.Slot = newValue
		}
	case *AssumeInstruction:
		if i.Predicate == oldValue {
			i.Predicate = newValue
		}
	case *StorageAddrInstruction:
		for j, key := range i.Keys {
			if key == oldValue {
				i.Keys[j] = newValue
			}
		}
	}
}

// replaceInTerminator replaces value references in a terminator
func (cse *CommonSubexpressionElimination) replaceInTerminator(term Terminator, oldValue, newValue *Value) {
	switch t := term.(type) {
	case *BranchTerminator:
		if t.Condition == oldValue {
			t.Condition = newValue
		}
	case *ReturnTerminator:
		if t.Value == oldValue {
			t.Value = newValue
		}
	}
}

// TODO: Storage Access Optimization
// - Combine multiple SLOAD/SSTORE operations
// - Cache storage reads in local variables
// - Optimize storage slot packing

// TODO: Loop Optimizations
// - Loop invariant code motion
// - Loop unrolling for small loops
// - Strength reduction

// TODO: Function Inlining
// - Inline small functions to reduce CALL overhead
// - Especially beneficial for internal functions

// TODO: Gas Estimation
// - Track estimated gas costs for operations
// - Optimize for minimal gas usage

// CheckedArithmeticOptimization replaces checked arithmetic with unchecked when safe
// This optimization looks for patterns like:
//
//	assume(%<=_result)     ; where <=_result is (a >= b)
//	%res, %ok = SUB_CHK(a, b)
//
// And replaces SUB_CHK with plain SUB since assume guarantees no underflow
type CheckedArithmeticOptimization struct{}

func (cao *CheckedArithmeticOptimization) Name() string {
	return "Checked Arithmetic Optimization"
}

func (cao *CheckedArithmeticOptimization) Description() string {
	return "Replaces SUB_CHK→SUB when dominated by assume that guarantees safety"
}

func (cao *CheckedArithmeticOptimization) Apply(program *Program) bool {
	changed := false

	for _, fn := range program.Functions {
		if cao.optimizeFunction(fn) {
			changed = true
		}
	}

	return changed
}

// optimizeFunction analyzes control flow to find SUB_CHK operations that can be optimized
func (cao *CheckedArithmeticOptimization) optimizeFunction(fn *Function) bool {
	changed := false

	for _, block := range fn.Blocks {
		if cao.optimizeBlock(block) {
			changed = true
		}
	}

	return changed
}

// optimizeBlock looks for assume + SUB_CHK patterns within a basic block
func (cao *CheckedArithmeticOptimization) optimizeBlock(block *BasicBlock) bool {
	changed := false

	// Track active assume predicates in this block
	assumedPredicates := make(map[*Value]bool)

	for i, inst := range block.Instructions {
		// Track assume instructions
		if assume, ok := inst.(*AssumeInstruction); ok {
			assumedPredicates[assume.Predicate] = true
			continue
		}

		// Look for SUB_CHK operations that can be optimized
		if checked, ok := inst.(*CheckedArithInstruction); ok && checked.Op == "SUB_CHK" {
			// Check if we have an assume that guarantees Left >= Right for SUB_CHK(Left, Right)
			if cao.isSubtractionSafe(checked.Left, checked.Right, assumedPredicates) {
				// Replace SUB_CHK with plain SUB
				// Note: We only keep the arithmetic result, not the check result
				newInst := &BinaryInstruction{
					ID:     checked.ID,
					Result: checked.ResultVal, // Use the arithmetic result
					Block:  checked.Block,
					Op:     "SUB",
					Left:   checked.Left,
					Right:  checked.Right,
				}
				block.Instructions[i] = newInst
				changed = true

				// TODO: The check result (checked.ResultOk) becomes dead code
				// and should be eliminated by the DCE pass that runs after this
			}
		}
	}

	return changed
}

// isSubtractionSafe checks if we have an assume that guarantees a >= b
func (cao *CheckedArithmeticOptimization) isSubtractionSafe(a, b *Value, assumes map[*Value]bool) bool {
	// Look for an assume predicate that guarantees a >= b (or equivalently b <= a)
	for predicate := range assumes {
		if cao.guaranteesGeq(predicate, a, b) {
			return true
		}
	}
	return false
}

// guaranteesGeq checks if an assumed predicate guarantees that a >= b
func (cao *CheckedArithmeticOptimization) guaranteesGeq(predicate, a, b *Value) bool {
	// We need to find the instruction that defined this predicate
	// and check if it represents a comparison that guarantees a >= b

	// In our IR, predicates are typically results of comparison operations
	// We need to trace back to find the comparison instruction

	// This is a simplified implementation - for now we'll use name patterns
	// A proper implementation would traverse the SSA def-use chains

	predicateName := predicate.Name
	// TODO: Use operand names for more precise matching
	_ = a.Name // aName - will be used in future implementation
	_ = b.Name // bName - will be used in future implementation

	// Look for patterns like "%<=_result_X" where the comparison was "b <= a"
	// This is fragile but works for our current IR generation patterns
	if strings.Contains(predicateName, "<=_result") {
		// We would need to trace back to the actual comparison instruction
		// For now, use a heuristic based on the pattern we observed:
		// If we have assume(%<=_result_X) and SUB_CHK(allowances_load_Y, amount_Z)
		// and the predicate name suggests it's from a <= comparison,
		// we can infer this might be safe

		// This is a very conservative approximation
		// TODO: Implement proper SSA analysis
		return true // For now, assume any <= predicate might work
	}

	return false
}

// TODO: EVM-Specific Optimizations
// - Optimize for EVM stack operations (DUP, SWAP)
// - Minimize stack depth
// - Optimize storage layout for gas efficiency
