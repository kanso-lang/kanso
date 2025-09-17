package ir

import (
	"fmt"
	"strings"
)

// Printer provides pretty-printing for IR
type Printer struct {
	indent int
	output strings.Builder
}

// NewPrinter creates a new IR printer
func NewPrinter() *Printer {
	return &Printer{indent: 0}
}

// Print returns the string representation of an IR program
func Print(program *Program) string {
	p := NewPrinter()
	p.printProgram(program)
	return p.output.String()
}

// Helper methods

func (p *Printer) writeIndent() {
	for i := 0; i < p.indent; i++ {
		p.output.WriteString("  ")
	}
}

func (p *Printer) writeLine(format string, args ...interface{}) {
	p.writeIndent()
	p.output.WriteString(fmt.Sprintf(format, args...))
	p.output.WriteString("\n")
}

func (p *Printer) write(format string, args ...interface{}) {
	p.output.WriteString(fmt.Sprintf(format, args...))
}

// printProgram prints the entire IR program
func (p *Printer) printProgram(program *Program) {
	p.writeLine("CONTRACT %s (IR)", program.Contract)
	p.writeLine("")

	// Print storage layout
	if len(program.Storage) > 0 {
		p.writeLine("STORAGE LAYOUT:")
		p.indent++
		for _, slot := range program.Storage {
			typeStr := slot.Type.String()
			if slotsType, ok := slot.Type.(*SlotsType); ok {
				if tupleKey, isTuple := slotsType.KeyType.(*TupleType); isTuple {
					// Multi-key slots - tuple key is hashed as single operation
					keyTypes := make([]string, len(tupleKey.Elements))
					for i, elem := range tupleKey.Elements {
						keyTypes[i] = elem.String()
					}
					// Generate appropriate comment based on slot name
					comment := ""
					if slot.Name == "allowances" && len(tupleKey.Elements) == 2 {
						comment = "  ; owner -> spender -> amount"
					} else if len(tupleKey.Elements) == 2 {
						comment = fmt.Sprintf("  ; %s -> %s -> %s",
							p.getKeyName(tupleKey.Elements[0]),
							p.getKeyName(tupleKey.Elements[1]),
							p.getValueName(slotsType.ValueType))
					}
					p.writeLine("slot[%d] %-12s : Slots<(%s), %s>%s",
						slot.Slot, slot.Name,
						strings.Join(keyTypes, ", "), slotsType.ValueType.String(),
						comment)
				} else {
					// Single-key mapping
					p.writeLine("slot[%d] %-12s : Slots<%s, %s>",
						slot.Slot, slot.Name, slotsType.KeyType.String(), slotsType.ValueType.String())
				}
			} else {
				// Simple storage slot
				p.writeLine("slot[%d] %-12s : %s", slot.Slot, slot.Name, typeStr)
			}
		}
		p.indent--
		p.writeLine("")
	}

	// Print address constructors
	p.writeLine("; ---------- Address constructors (abstract addresses; lowered to keccak256 late) ----------")
	p.writeLine("; One-key mapping address:   balances[addr]")
	p.writeLine("SADDR_MAP1(base_slot, k1)               -> StorageAddr")
	p.writeLine("; Two-key mapping address:   allowances[owner][spender]")
	p.writeLine("SADDR_MAP2(base_slot, k1, k2)           -> StorageAddr")
	p.writeLine("")

	// Print events
	p.writeLine("; ---------- Events ----------")
	p.writeLine("; Event signatures (computed at compile time)")
	for _, eventSig := range program.EventSignatures {
		p.writeLine("%%%s = EVENT_SIG \"%s\"", eventSig.Name, eventSig.Signature)
	}
	p.writeLine("")
	p.writeLine("; ABI encoders: writes(Memory) - pure w.r.t Storage, can be hoisted/CSE'd")
	p.writeLine("; LOG operations: reads(Memory) + emits(Log) - only Storage-effectful event op")
	p.writeLine("EVENT Transfer(from: Address, to: Address, value: U256) -> LOG3")
	p.writeLine("EVENT Approval(owner: Address, spender: Address, value: U256) -> LOG3")
	p.writeLine("")

	// Print helpers
	p.writeLine("; ---------- Helpers ----------")
	p.writeLine("; Checked arithmetic (split value + predicate); DCE replaces with raw ops if dominated by assume(...)")
	p.writeLine("SUB_CHK(a:U256, b:U256) -> (res:U256, ok:Bool)    ; ok == (a >= b)")
	p.writeLine("ADD_CHK(a:U256, b:U256) -> (res:U256, ok:Bool)    ; ok == (a + b < 2^256)")
	p.writeLine("DIV_CHK(a:U256, b:U256) -> (res:U256, ok:Bool)    ; ok == (b != 0)")
	p.writeLine("")
	p.writeLine("; Branch with explicit path fact on the taken \"ok\" edge.")
	p.writeLine("br_if(cond: Bool, then: Label, else: Label)")
	p.writeLine("assume(pred: Bool)")
	p.writeLine("")
	p.writeLine("; =========================================================================================")
	p.writeLine("")

	// Print constants
	if len(program.Constants) > 0 {
		p.writeLine("CONSTANTS:")
		p.indent++
		for _, constant := range program.Constants {
			p.writeLine("%s = %v", p.valueString(constant.Value), constant.Data)
		}
		p.indent--
		p.writeLine("")
	}

	// Print functions
	for _, fn := range program.Functions {
		p.printFunction(fn)
		p.writeLine("")
	}

	// Print control flow graph info
	if program.CFG != nil {
		p.printCFG(program.CFG)
	}
}

// printFunction prints an SSA function
func (p *Printer) printFunction(fn *Function) {
	// Function signature
	sig := fmt.Sprintf("FUNCTION %s(", fn.Name)
	for i, param := range fn.Params {
		if i > 0 {
			sig += ", "
		}
		sig += fmt.Sprintf("%s: %s", param.Name, param.Type.String())
	}
	sig += ")"

	if fn.ReturnType != nil {
		sig += " -> " + fn.ReturnType.String()
	}

	// Add metadata in brackets
	var metadata []string
	if fn.Create {
		metadata = append(metadata, "create")
	}
	if fn.External {
		metadata = append(metadata, "external")
	}
	if len(fn.Reads) > 0 {
		metadata = append(metadata, fmt.Sprintf("reads(%s)", strings.Join(fn.Reads, ", ")))
	}
	if len(fn.Writes) > 0 {
		metadata = append(metadata, fmt.Sprintf("writes(%s)", strings.Join(fn.Writes, ", ")))
	}

	p.writeLine("%s", sig)
	if len(metadata) > 0 {
		p.writeLine("  [%s]", strings.Join(metadata, ", "))
	}
	p.writeLine("{")

	// Print basic blocks
	for _, block := range fn.Blocks {
		p.printBasicBlock(block)
	}

	p.writeLine("}")
}

// printBasicBlock prints a basic block in IR form
func (p *Printer) printBasicBlock(block *BasicBlock) {
	p.writeLine("%s:", block.Label)

	// Print instructions with simple indentation
	for _, inst := range block.Instructions {
		p.write("  ")
		p.printInstructionSimple(inst)
		p.writeLine("")
	}

	// Print terminator
	if block.Terminator != nil {
		p.write("  ")
		p.printInstructionSimple(block.Terminator)
		p.writeLine("")
	}
}

// printInstruction prints an IR instruction
func (p *Printer) printInstruction(inst Instruction) {
	switch i := inst.(type) {
	case *PhiInstruction:
		p.printPhi(i)
	case *LoadInstruction:
		p.writeLine("%s = LOAD %s", p.valueString(i.Result), p.valueString(i.Address))
	case *StoreInstruction:
		p.writeLine("STORE %s, %s", p.valueString(i.Address), p.valueString(i.Value))
	case *StorageLoadInstruction:
		if i.SlotNum >= 0 {
			p.writeLine("%s = SLOAD slot[%d]", p.valueString(i.Result), i.SlotNum)
		} else {
			p.writeLine("%s = SLOAD %s", p.valueString(i.Result), p.valueString(i.Slot))
		}
	case *StorageStoreInstruction:
		if i.SlotNum >= 0 {
			p.writeLine("SSTORE slot[%d]:%s, %s", i.SlotNum, i.Type.String(), p.valueString(i.Value))
		} else {
			p.writeLine("SSTORE %s, %s", p.valueString(i.Slot), p.valueString(i.Value))
		}
	case *KeyedStorageLoadInstruction:
		p.writeLine("%s = SLOAD keccak256(%s . %d) ; keyed storage access",
			p.valueString(i.Result), p.valueString(i.Key), i.BaseSlot)
	case *KeyedStorageStoreInstruction:
		p.writeLine("SSTORE keccak256(%s . %d), %s ; keyed storage access",
			p.valueString(i.Key), i.BaseSlot, p.valueString(i.Value))
	case *BinaryInstruction:
		p.writeLine("%s = %s %s, %s",
			p.valueString(i.Result), i.Op, p.valueString(i.Left), p.valueString(i.Right))
	case *CallInstruction:
		args := make([]string, len(i.Args))
		for j, arg := range i.Args {
			args[j] = p.valueString(arg)
		}
		funcName := i.Function
		if i.Module != "" {
			funcName = i.Module + "::" + i.Function
		}
		if i.Result != nil {
			p.writeLine("%s = %s(%s)", p.valueString(i.Result), funcName, strings.Join(args, ", "))
		} else {
			p.writeLine("%s(%s)", funcName, strings.Join(args, ", "))
		}
	case *ConstantInstruction:
		p.writeLine("%s = CONST %v:%s", p.valueString(i.Result), i.Value, i.Type.String())
	case *SenderInstruction:
		p.writeLine("%s = std::evm::sender()", p.valueString(i.Result))
	case *EmitInstruction:
		args := make([]string, len(i.Args))
		for j, arg := range i.Args {
			args[j] = p.valueString(arg)
		}
		p.writeLine("std::evm::emit(%s{%s})", i.Event, strings.Join(args, ", "))
	case *RequireInstruction:
		p.writeLine("require!(%s, %s)", p.valueString(i.Condition), p.valueString(i.Error))
	case *ReturnTerminator:
		if i.Value != nil {
			p.writeLine("RETURN %s", p.valueString(i.Value))
		} else {
			p.writeLine("RETURN")
		}
	case *BranchTerminator:
		p.writeLine("BRANCH %s ? %s : %s",
			p.valueString(i.Condition), i.TrueBlock.Label, i.FalseBlock.Label)
	case *JumpTerminator:
		p.writeLine("JUMP %s", i.Target.Label)
	case *StorageAddrInstruction:
		if len(i.Keys) == 1 {
			p.writeLine("%s = SADDR_MAP1(%d, %s)",
				p.valueString(i.Result), i.BaseSlot, p.valueString(i.Keys[0]))
		} else if len(i.Keys) == 2 {
			p.writeLine("%s = SADDR_MAP2(%d, %s, %s)",
				p.valueString(i.Result), i.BaseSlot, p.valueString(i.Keys[0]), p.valueString(i.Keys[1]))
		} else {
			// Fallback for more than 2 keys
			keyStrs := make([]string, len(i.Keys))
			for j, key := range i.Keys {
				keyStrs[j] = p.valueString(key)
			}
			p.writeLine("%s = SADDR_MAP(%d, %s)",
				p.valueString(i.Result), i.BaseSlot, strings.Join(keyStrs, ", "))
		}
	case *CheckedArithInstruction:
		p.writeLine("%s, %s = %s %s, %s",
			p.valueString(i.ResultVal), p.valueString(i.ResultOk), i.Op,
			p.valueString(i.Left), p.valueString(i.Right))
	case *AssumeInstruction:
		p.writeLine("assume(%s)", p.valueString(i.Predicate))
	case *EventSignatureInstruction:
		p.writeLine("%s = EVENT_SIG \"%s\"", p.valueString(i.Result), i.Signature)
	case *TopicAddrInstruction:
		p.writeLine("%s = TOPIC_ADDR %s", p.valueString(i.Result), p.valueString(i.Address))
	case *ABIEncU256Instruction:
		p.writeLine("%s,%s = ABI_ENC_U256 %s",
			p.valueString(i.ResultData),
			p.valueString(i.ResultLen),
			p.valueString(i.Value))
	case *LogInstruction:
		// Build the LOG instruction with proper format: LOG3 %sig, %t1, %t2, %dp, %dl
		args := []string{p.valueString(i.Signature)}
		for _, topic := range i.TopicArgs {
			args = append(args, p.valueString(topic))
		}
		if i.DataPtr != nil {
			args = append(args, p.valueString(i.DataPtr))
		}
		if i.DataLen != nil {
			args = append(args, p.valueString(i.DataLen))
		}
		p.writeLine("LOG%d %s", i.Topics, strings.Join(args, ", "))
	case *RevertInstruction:
		p.writeLine("REVERT")
	default:
		p.writeLine("UNKNOWN_INST<%T> %d", i, i.GetID())
	}
}

// printPhi prints a phi instruction
func (p *Printer) printPhi(phi *PhiInstruction) {
	inputs := make([]string, 0, len(phi.Inputs))
	for block, value := range phi.Inputs {
		inputs = append(inputs, fmt.Sprintf("[%s: %s]", block.Label, p.valueString(value)))
	}
	p.writeLine("%s = PHI %s", p.valueString(phi.Result), strings.Join(inputs, ", "))
}

// printCFG prints control flow graph information
func (p *Printer) printCFG(cfg *ControlFlowGraph) {
	p.writeLine("CONTROL FLOW GRAPH:")
	p.indent++

	// Print overall summary
	if len(cfg.EntryPoints) > 0 {
		entryLabels := make([]string, len(cfg.EntryPoints))
		for i, entry := range cfg.EntryPoints {
			entryLabels[i] = entry.Label
		}
		p.writeLine("Total Entry Points: [%s]", strings.Join(entryLabels, ", "))
	}

	p.writeLine("")

	// Print per-function CFG information
	for funcName, fnCFG := range cfg.Functions {
		p.writeLine("Function: %s", funcName)
		p.indent++

		// Entry point
		if fnCFG.Entry != nil {
			p.writeLine("Entry: %s", fnCFG.Entry.Label)
		}

		// Success exits
		if len(fnCFG.SuccessExits) > 0 {
			successLabels := make([]string, len(fnCFG.SuccessExits))
			for i, exit := range fnCFG.SuccessExits {
				successLabels[i] = exit.Label
			}
			p.writeLine("Success Exits: [%s]", strings.Join(successLabels, ", "))
		}

		// Failure exits
		if len(fnCFG.FailureExits) > 0 {
			failureLabels := make([]string, len(fnCFG.FailureExits))
			for i, exit := range fnCFG.FailureExits {
				failureLabels[i] = exit.Label
			}
			p.writeLine("Failure Exits: [%s]", strings.Join(failureLabels, ", "))
		}

		// Block count for this function
		p.writeLine("Blocks: %d", len(fnCFG.Blocks))

		p.indent--
		p.writeLine("")
	}

	// Print total block count
	p.writeLine("Total Blocks: %d", len(cfg.Blocks))

	// Print block relationships
	if len(cfg.Blocks) > 0 {
		p.writeLine("Block Relationships:")
		p.indent++
		for _, block := range cfg.Blocks {
			// Show each block with its successors
			if len(block.Successors) > 0 {
				successorLabels := make([]string, len(block.Successors))
				for i, succ := range block.Successors {
					successorLabels[i] = succ.Label
				}
				p.writeLine("%s -> %s", block.Label, strings.Join(successorLabels, ", "))
			} else {
				// Terminal block (no successors)
				p.writeLine("%s -> [END]", block.Label)
			}
		}
		p.indent--
	}

	if len(cfg.Loops) > 0 {
		p.writeLine("Loops:")
		p.indent++
		for i, loop := range cfg.Loops {
			p.writeLine("Loop %d: header=%s, exits=%s",
				i, loop.Header.Label, p.blockLabels(loop.Exits))
			if len(loop.Invariant) > 0 {
				invariants := make([]string, len(loop.Invariant))
				for j, inv := range loop.Invariant {
					invariants[j] = p.valueString(inv)
				}
				p.writeLine("  Invariants: %s", strings.Join(invariants, ", "))
			}
		}
		p.indent--
	}

	p.indent--
}

// printInstructionSimple prints an instruction in simplified format
func (p *Printer) printInstructionSimple(inst Instruction) {
	switch i := inst.(type) {
	case *ConstantInstruction:
		// Skip constant instructions - they're implicit in the values
		return
	case *StorageStoreInstruction:
		if i.SlotNum >= 0 {
			p.write("SSTORE slot[%d], %s", i.SlotNum, p.valueString(i.Value))
		} else {
			p.write("SSTORE %s, %s", p.valueString(i.Slot), p.valueString(i.Value))
		}
	case *StorageLoadInstruction:
		if i.SlotNum >= 0 {
			p.write("%s = SLOAD slot[%d]", p.valueString(i.Result), i.SlotNum)
		} else {
			p.write("%s = SLOAD %s", p.valueString(i.Result), p.valueString(i.Slot))
		}
	case *StorageAddrInstruction:
		if len(i.Keys) == 1 {
			p.write("%s = SADDR_MAP1(%d, %s)", p.valueString(i.Result), i.BaseSlot, p.valueString(i.Keys[0]))
		} else if len(i.Keys) == 2 {
			p.write("%s = SADDR_MAP2(%d, %s, %s)", p.valueString(i.Result), i.BaseSlot, p.valueString(i.Keys[0]), p.valueString(i.Keys[1]))
		}
	case *CheckedArithInstruction:
		p.write("%s, %s = %s(%s, %s)", p.valueString(i.ResultVal), p.valueString(i.ResultOk), i.Op, p.valueString(i.Left), p.valueString(i.Right))
	case *BinaryInstruction:
		p.write("%s = %s %s, %s", p.valueString(i.Result), i.Op, p.valueString(i.Left), p.valueString(i.Right))
	case *CallInstruction:
		funcName := i.Function
		if i.Module != "" {
			funcName = i.Module + "::" + i.Function
		}

		// Check if it's a local function (no module)
		if i.Module == "" && (i.Function == "mint" || i.Function == "do_transfer") {
			// Local functions use 'call' prefix
			p.write("call %s(%s)", i.Function, p.argsString(i.Args))
		} else if i.Result != nil {
			p.write("%s = %s(%s)", p.valueString(i.Result), funcName, p.argsString(i.Args))
		} else {
			// For void functions like emit
			p.write("%s(%s)", funcName, p.argsString(i.Args))
		}
	case *SenderInstruction:
		p.write("%s = std::evm::sender()", p.valueString(i.Result))
	case *StoreInstruction:
		// Local variable store - skip for cleaner output
		return
	case *AssumeInstruction:
		p.write("assume(%s)", p.valueString(i.Predicate))
	case *EventSignatureInstruction:
		p.write("%s = EVENT_SIG \"%s\"", p.valueString(i.Result), i.Signature)
	case *TopicAddrInstruction:
		p.write("%s = TOPIC_ADDR %s", p.valueString(i.Result), p.valueString(i.Address))
	case *ABIEncU256Instruction:
		// Print ABI encoding with effect information
		effectStr := p.formatInstructionEffects(i.GetEffects())
		if i.MemoryRegion != nil {
			p.write("%s,%s = ABI_ENC_U256 %s  ; %s, region=%s",
				p.valueString(i.ResultData), p.valueString(i.ResultLen),
				p.valueString(i.Value), effectStr, i.MemoryRegion.Name)
		} else {
			p.write("%s,%s = ABI_ENC_U256 %s  ; %s",
				p.valueString(i.ResultData), p.valueString(i.ResultLen),
				p.valueString(i.Value), effectStr)
		}
	case *LogInstruction:
		// Show as proper LOG instruction with effects
		args := []string{p.valueString(i.Signature)}
		for _, topic := range i.TopicArgs {
			args = append(args, p.valueString(topic))
		}
		if i.DataPtr != nil {
			args = append(args, p.valueString(i.DataPtr))
		}
		if i.DataLen != nil {
			args = append(args, p.valueString(i.DataLen))
		}
		effectStr := p.formatInstructionEffects(i.GetEffects())
		p.write("LOG%d %s  ; %s", i.Topics, strings.Join(args, ", "), effectStr)
	case *RevertInstruction:
		p.write("REVERT")
	case *BranchTerminator:
		p.write("br_if(%s, %s, %s)", p.valueString(i.Condition), i.TrueBlock.Label, i.FalseBlock.Label)
	case *ReturnTerminator:
		if i.Value != nil {
			p.write("RETURN %s", p.valueString(i.Value))
		} else {
			p.write("RETURN")
		}
	default:
		p.write("UNKNOWN_INST<%T>", i)
	}
}

// Helper methods

// valueString returns a string representation of an IR value
func (p *Printer) valueString(value *Value) string {
	if value == nil {
		return "null"
	}
	return fmt.Sprintf("%%%s", value.Name)
}

// argsString formats function arguments
func (p *Printer) argsString(args []*Value) string {
	if len(args) == 0 {
		return ""
	}
	argStrs := make([]string, len(args))
	for i, arg := range args {
		argStrs[i] = p.valueString(arg)
	}
	return strings.Join(argStrs, ", ")
}

// formatMemoryEffects formats memory effects for display
func (p *Printer) formatMemoryEffects(effects []MemoryEffect) string {
	if len(effects) == 0 {
		return "none"
	}

	var effectStrs []string
	for _, effect := range effects {
		effectStrs = append(effectStrs, string(effect.Type))
	}

	return strings.Join(effectStrs, ",")
}

// formatInstructionEffects formats instruction effects for display
func (p *Printer) formatInstructionEffects(effects []Effect) string {
	if len(effects) == 0 {
		return "pure"
	}

	var effectStrs []string
	for _, effect := range effects {
		switch e := effect.(type) {
		case *StorageEffect:
			if e.Type == "log" {
				effectStrs = append(effectStrs, "emits(Log)")
			} else {
				effectStrs = append(effectStrs, fmt.Sprintf("%s(Storage)", e.Type))
			}
		case *MemoryEffectOp:
			effectStrs = append(effectStrs, fmt.Sprintf("%s(Memory)", e.Type))
		case *PureEffect:
			effectStrs = append(effectStrs, "pure")
		}
	}

	return strings.Join(effectStrs, ", ")
}

// getKeyName returns a descriptive name for key types
func (p *Printer) getKeyName(keyType Type) string {
	switch keyType.(type) {
	case *AddressType:
		return "addr"
	default:
		return "key"
	}
}

// getValueName returns a descriptive name for value types
func (p *Printer) getValueName(valueType Type) string {
	switch valueType.(type) {
	case *IntType:
		return "amount"
	default:
		return "value"
	}
}

// blockLabels returns a comma-separated list of block labels
func (p *Printer) blockLabels(blocks []*BasicBlock) string {
	if len(blocks) == 0 {
		return "none"
	}
	labels := make([]string, len(blocks))
	for i, block := range blocks {
		labels[i] = block.Label
	}
	return strings.Join(labels, ", ")
}

// String methods for debugging

func (p *Program) String() string    { return Print(p) }
func (f *Function) String() string   { return "IR Function: " + f.Name }
func (b *BasicBlock) String() string { return "BasicBlock: " + b.Label }
func (v *Value) String() string      { return fmt.Sprintf("%%%s_%d:%s", v.Name, v.ID, v.Type.String()) }

// String methods for instructions
func (p *PhiInstruction) String() string               { return fmt.Sprintf("PHI %d", p.ID) }
func (l *LoadInstruction) String() string              { return fmt.Sprintf("LOAD %d", l.ID) }
func (s *StoreInstruction) String() string             { return fmt.Sprintf("STORE %d", s.ID) }
func (s *StorageLoadInstruction) String() string       { return fmt.Sprintf("SLOAD %d", s.ID) }
func (s *StorageStoreInstruction) String() string      { return fmt.Sprintf("SSTORE %d", s.ID) }
func (k *KeyedStorageLoadInstruction) String() string  { return fmt.Sprintf("KEYED_SLOAD %d", k.ID) }
func (k *KeyedStorageStoreInstruction) String() string { return fmt.Sprintf("KEYED_SSTORE %d", k.ID) }
func (b *BinaryInstruction) String() string            { return fmt.Sprintf("BINARY %d", b.ID) }
func (c *CallInstruction) String() string              { return fmt.Sprintf("CALL %d", c.ID) }
func (c *ConstantInstruction) String() string          { return fmt.Sprintf("CONST %d", c.ID) }
func (s *SenderInstruction) String() string            { return fmt.Sprintf("SENDER %d", s.ID) }
func (e *EmitInstruction) String() string              { return fmt.Sprintf("EMIT %d", e.ID) }
func (r *RequireInstruction) String() string           { return fmt.Sprintf("REQUIRE %d", r.ID) }

// Enhanced instruction String methods
func (s *StorageAddrInstruction) String() string  { return fmt.Sprintf("SADDR %d", s.ID) }
func (c *CheckedArithInstruction) String() string { return fmt.Sprintf("%s %d", c.Op, c.ID) }
func (a *AssumeInstruction) String() string       { return fmt.Sprintf("ASSUME %d", a.ID) }
func (e *EventSignatureInstruction) String() string {
	return fmt.Sprintf("EVENT_SIG %d", e.ID)
}
func (t *TopicAddrInstruction) String() string {
	return fmt.Sprintf("TOPIC_ADDR %d", t.ID)
}
func (a *ABIEncU256Instruction) String() string {
	return fmt.Sprintf("ABI_ENC_U256 %d", a.ID)
}
func (l *LogInstruction) String() string    { return fmt.Sprintf("LOG%d %d", l.Topics, l.ID) }
func (r *RevertInstruction) String() string { return fmt.Sprintf("REVERT %d", r.ID) }

func (r *ReturnTerminator) String() string { return fmt.Sprintf("RETURN %d", r.ID) }
func (b *BranchTerminator) String() string { return fmt.Sprintf("BRANCH %d", b.ID) }
func (j *JumpTerminator) String() string   { return fmt.Sprintf("JUMP %d", j.ID) }
