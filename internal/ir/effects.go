package ir

// This file implements the GetEffects() method for all instruction types
// Effects describe the side effects of instructions (Storage, Memory, Pure)

// PhiInstruction effects
func (i *PhiInstruction) GetEffects() []Effect {
	return []Effect{&PureEffect{}}
}

// LoadInstruction effects
func (i *LoadInstruction) GetEffects() []Effect {
	return []Effect{&MemoryEffectOp{Type: MemoryEffectRead, Region: nil}}
}

// StoreInstruction effects
func (i *StoreInstruction) GetEffects() []Effect {
	return []Effect{&MemoryEffectOp{Type: MemoryEffectWrite, Region: nil}}
}

// StorageLoadInstruction effects
func (i *StorageLoadInstruction) GetEffects() []Effect {
	return []Effect{&StorageEffect{Type: "read", Slot: i.SlotNum}}
}

// StorageStoreInstruction effects
func (i *StorageStoreInstruction) GetEffects() []Effect {
	return []Effect{&StorageEffect{Type: "write", Slot: i.SlotNum}}
}

// KeyedStorageLoadInstruction effects
func (i *KeyedStorageLoadInstruction) GetEffects() []Effect {
	return []Effect{&StorageEffect{Type: "read", Slot: -1}}
}

// KeyedStorageStoreInstruction effects
func (i *KeyedStorageStoreInstruction) GetEffects() []Effect {
	return []Effect{&StorageEffect{Type: "write", Slot: -1}}
}

// BinaryInstruction effects
func (i *BinaryInstruction) GetEffects() []Effect {
	return []Effect{&PureEffect{}}
}

// CallInstruction effects (depends on the function being called)
func (i *CallInstruction) GetEffects() []Effect {
	// For now, assume function calls can have any effect
	// In the future, this should analyze the called function
	return []Effect{&StorageEffect{Type: "read", Slot: -1}, &StorageEffect{Type: "write", Slot: -1}}
}

// ConstantInstruction effects
func (i *ConstantInstruction) GetEffects() []Effect {
	return []Effect{&PureEffect{}}
}

// SenderInstruction effects
func (i *SenderInstruction) GetEffects() []Effect {
	// Reading transaction context is pure in terms of storage/memory
	return []Effect{&PureEffect{}}
}

// EmitInstruction effects
func (i *EmitInstruction) GetEffects() []Effect {
	// Events are logged but don't affect storage/memory
	return []Effect{&PureEffect{}}
}

// RequireInstruction effects
func (i *RequireInstruction) GetEffects() []Effect {
	// Require can revert but doesn't write to storage/memory
	return []Effect{&PureEffect{}}
}

// StorageAddrInstruction effects
func (i *StorageAddrInstruction) GetEffects() []Effect {
	// Computing storage addresses is pure
	return []Effect{&PureEffect{}}
}

// CheckedArithInstruction effects
func (i *CheckedArithInstruction) GetEffects() []Effect {
	return []Effect{&PureEffect{}}
}

// AssumeInstruction effects
func (i *AssumeInstruction) GetEffects() []Effect {
	return []Effect{&PureEffect{}}
}

// LogInstruction effects
func (i *LogInstruction) GetEffects() []Effect {
	// LOG operations emit events to the blockchain log
	// They consume memory data but don't modify storage
	// They are the only Storage-effectful operation for events
	return []Effect{
		&MemoryEffectOp{Type: MemoryEffectRead, Region: nil}, // Reads event data from memory
		&StorageEffect{Type: "log", Slot: -1},                // Emits to blockchain log
	}
}

// TopicAddrInstruction effects
func (i *TopicAddrInstruction) GetEffects() []Effect {
	// Converting addresses to topics is pure
	return []Effect{&PureEffect{}}
}

// ABIEncU256Instruction effects - THIS IS THE KEY ONE
func (i *ABIEncU256Instruction) GetEffects() []Effect {
	// ABI encoding writes to memory
	effects := []Effect{}

	// If we have a memory region, use it for precise effect tracking
	if i.MemoryRegion != nil {
		effects = append(effects, &MemoryEffectOp{
			Type:   MemoryEffectWrite,
			Region: i.MemoryRegion,
		})
	} else {
		// Fallback: general memory write
		effects = append(effects, &MemoryEffectOp{
			Type:   MemoryEffectWrite,
			Region: nil,
		})
	}

	return effects
}

// EventSignatureInstruction effects
func (i *EventSignatureInstruction) GetEffects() []Effect {
	// Computing event signatures is pure
	return []Effect{&PureEffect{}}
}

// RevertInstruction effects
func (i *RevertInstruction) GetEffects() []Effect {
	// Revert rolls back all changes
	return []Effect{&StorageEffect{Type: "revert", Slot: -1}}
}

// Terminator effects

// ReturnTerminator effects
func (t *ReturnTerminator) GetEffects() []Effect {
	return []Effect{&PureEffect{}}
}

// BranchTerminator effects
func (t *BranchTerminator) GetEffects() []Effect {
	return []Effect{&PureEffect{}}
}

// JumpTerminator effects
func (t *JumpTerminator) GetEffects() []Effect {
	return []Effect{&PureEffect{}}
}
