package ir

import (
	"testing"
)

func TestPhiInstructionEffects(t *testing.T) {
	phi := &PhiInstruction{}
	effects := phi.GetEffects()

	if len(effects) != 1 {
		t.Errorf("PhiInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("PhiInstruction should have PureEffect")
	}
}

func TestLoadInstructionEffects(t *testing.T) {
	load := &LoadInstruction{}
	effects := load.GetEffects()

	if len(effects) != 1 {
		t.Errorf("LoadInstruction should have 1 effect, got %d", len(effects))
	}

	if memEffect, ok := effects[0].(*MemoryEffectOp); ok {
		if memEffect.Type != MemoryEffectRead {
			t.Error("LoadInstruction should have MemoryEffectRead")
		}
	} else {
		t.Error("LoadInstruction should have MemoryEffectOp")
	}
}

func TestStoreInstructionEffects(t *testing.T) {
	store := &StoreInstruction{}
	effects := store.GetEffects()

	if len(effects) != 1 {
		t.Errorf("StoreInstruction should have 1 effect, got %d", len(effects))
	}

	if memEffect, ok := effects[0].(*MemoryEffectOp); ok {
		if memEffect.Type != MemoryEffectWrite {
			t.Error("StoreInstruction should have MemoryEffectWrite")
		}
	} else {
		t.Error("StoreInstruction should have MemoryEffectOp")
	}
}

func TestStorageLoadInstructionEffects(t *testing.T) {
	storageLoad := &StorageLoadInstruction{SlotNum: 5}
	effects := storageLoad.GetEffects()

	if len(effects) != 1 {
		t.Errorf("StorageLoadInstruction should have 1 effect, got %d", len(effects))
	}

	if storageEffect, ok := effects[0].(*StorageEffect); ok {
		if storageEffect.Type != "read" {
			t.Error("StorageLoadInstruction should have read effect")
		}
		if storageEffect.Slot != 5 {
			t.Errorf("StorageLoadInstruction should have slot 5, got %d", storageEffect.Slot)
		}
	} else {
		t.Error("StorageLoadInstruction should have StorageEffect")
	}
}

func TestStorageStoreInstructionEffects(t *testing.T) {
	storageStore := &StorageStoreInstruction{SlotNum: 3}
	effects := storageStore.GetEffects()

	if len(effects) != 1 {
		t.Errorf("StorageStoreInstruction should have 1 effect, got %d", len(effects))
	}

	if storageEffect, ok := effects[0].(*StorageEffect); ok {
		if storageEffect.Type != "write" {
			t.Error("StorageStoreInstruction should have write effect")
		}
		if storageEffect.Slot != 3 {
			t.Errorf("StorageStoreInstruction should have slot 3, got %d", storageEffect.Slot)
		}
	} else {
		t.Error("StorageStoreInstruction should have StorageEffect")
	}
}

func TestKeyedStorageLoadInstructionEffects(t *testing.T) {
	keyedLoad := &KeyedStorageLoadInstruction{}
	effects := keyedLoad.GetEffects()

	if len(effects) != 1 {
		t.Errorf("KeyedStorageLoadInstruction should have 1 effect, got %d", len(effects))
	}

	if storageEffect, ok := effects[0].(*StorageEffect); ok {
		if storageEffect.Type != "read" {
			t.Error("KeyedStorageLoadInstruction should have read effect")
		}
		if storageEffect.Slot != -1 {
			t.Errorf("KeyedStorageLoadInstruction should have slot -1, got %d", storageEffect.Slot)
		}
	} else {
		t.Error("KeyedStorageLoadInstruction should have StorageEffect")
	}
}

func TestKeyedStorageStoreInstructionEffects(t *testing.T) {
	keyedStore := &KeyedStorageStoreInstruction{}
	effects := keyedStore.GetEffects()

	if len(effects) != 1 {
		t.Errorf("KeyedStorageStoreInstruction should have 1 effect, got %d", len(effects))
	}

	if storageEffect, ok := effects[0].(*StorageEffect); ok {
		if storageEffect.Type != "write" {
			t.Error("KeyedStorageStoreInstruction should have write effect")
		}
		if storageEffect.Slot != -1 {
			t.Errorf("KeyedStorageStoreInstruction should have slot -1, got %d", storageEffect.Slot)
		}
	} else {
		t.Error("KeyedStorageStoreInstruction should have StorageEffect")
	}
}

func TestBinaryInstructionEffects(t *testing.T) {
	binary := &BinaryInstruction{}
	effects := binary.GetEffects()

	if len(effects) != 1 {
		t.Errorf("BinaryInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("BinaryInstruction should have PureEffect")
	}
}

func TestCallInstructionEffects(t *testing.T) {
	call := &CallInstruction{}
	effects := call.GetEffects()

	if len(effects) != 2 {
		t.Errorf("CallInstruction should have 2 effects, got %d", len(effects))
	}

	// Should have both read and write storage effects
	hasReadEffect := false
	hasWriteEffect := false

	for _, effect := range effects {
		if storageEffect, ok := effect.(*StorageEffect); ok {
			if storageEffect.Type == "read" && storageEffect.Slot == -1 {
				hasReadEffect = true
			}
			if storageEffect.Type == "write" && storageEffect.Slot == -1 {
				hasWriteEffect = true
			}
		}
	}

	if !hasReadEffect {
		t.Error("CallInstruction should have storage read effect")
	}
	if !hasWriteEffect {
		t.Error("CallInstruction should have storage write effect")
	}
}

func TestConstantInstructionEffects(t *testing.T) {
	constant := &ConstantInstruction{}
	effects := constant.GetEffects()

	if len(effects) != 1 {
		t.Errorf("ConstantInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("ConstantInstruction should have PureEffect")
	}
}

func TestSenderInstructionEffects(t *testing.T) {
	sender := &SenderInstruction{}
	effects := sender.GetEffects()

	if len(effects) != 1 {
		t.Errorf("SenderInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("SenderInstruction should have PureEffect")
	}
}

func TestEmitInstructionEffects(t *testing.T) {
	emit := &EmitInstruction{}
	effects := emit.GetEffects()

	if len(effects) != 1 {
		t.Errorf("EmitInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("EmitInstruction should have PureEffect")
	}
}

func TestRequireInstructionEffects(t *testing.T) {
	require := &RequireInstruction{}
	effects := require.GetEffects()

	if len(effects) != 1 {
		t.Errorf("RequireInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("RequireInstruction should have PureEffect")
	}
}

func TestStorageAddrInstructionEffects(t *testing.T) {
	storageAddr := &StorageAddrInstruction{}
	effects := storageAddr.GetEffects()

	if len(effects) != 1 {
		t.Errorf("StorageAddrInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("StorageAddrInstruction should have PureEffect")
	}
}

func TestCheckedArithInstructionEffects(t *testing.T) {
	checkedArith := &CheckedArithInstruction{}
	effects := checkedArith.GetEffects()

	if len(effects) != 1 {
		t.Errorf("CheckedArithInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("CheckedArithInstruction should have PureEffect")
	}
}

func TestAssumeInstructionEffects(t *testing.T) {
	assume := &AssumeInstruction{}
	effects := assume.GetEffects()

	if len(effects) != 1 {
		t.Errorf("AssumeInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("AssumeInstruction should have PureEffect")
	}
}

func TestLogInstructionEffects(t *testing.T) {
	log := &LogInstruction{}
	effects := log.GetEffects()

	if len(effects) != 2 {
		t.Errorf("LogInstruction should have 2 effects, got %d", len(effects))
	}

	hasMemoryRead := false
	hasStorageLog := false

	for _, effect := range effects {
		switch e := effect.(type) {
		case *MemoryEffectOp:
			if e.Type == MemoryEffectRead {
				hasMemoryRead = true
			}
		case *StorageEffect:
			if e.Type == "log" && e.Slot == -1 {
				hasStorageLog = true
			}
		}
	}

	if !hasMemoryRead {
		t.Error("LogInstruction should have memory read effect")
	}
	if !hasStorageLog {
		t.Error("LogInstruction should have storage log effect")
	}
}

func TestTopicAddrInstructionEffects(t *testing.T) {
	topicAddr := &TopicAddrInstruction{}
	effects := topicAddr.GetEffects()

	if len(effects) != 1 {
		t.Errorf("TopicAddrInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("TopicAddrInstruction should have PureEffect")
	}
}

func TestABIEncU256InstructionEffects(t *testing.T) {
	// Test without memory region
	abiEnc := &ABIEncU256Instruction{}
	effects := abiEnc.GetEffects()

	if len(effects) != 1 {
		t.Errorf("ABIEncU256Instruction should have 1 effect, got %d", len(effects))
	}

	if memEffect, ok := effects[0].(*MemoryEffectOp); ok {
		if memEffect.Type != MemoryEffectWrite {
			t.Error("ABIEncU256Instruction should have MemoryEffectWrite")
		}
		if memEffect.Region != nil {
			t.Error("ABIEncU256Instruction without region should have nil region")
		}
	} else {
		t.Error("ABIEncU256Instruction should have MemoryEffectOp")
	}

	// Test with memory region
	region := &MemoryRegion{Name: "test_region"}
	abiEncWithRegion := &ABIEncU256Instruction{MemoryRegion: region}
	effectsWithRegion := abiEncWithRegion.GetEffects()

	if len(effectsWithRegion) != 1 {
		t.Errorf("ABIEncU256Instruction with region should have 1 effect, got %d", len(effectsWithRegion))
	}

	if memEffect, ok := effectsWithRegion[0].(*MemoryEffectOp); ok {
		if memEffect.Type != MemoryEffectWrite {
			t.Error("ABIEncU256Instruction with region should have MemoryEffectWrite")
		}
		if memEffect.Region != region {
			t.Error("ABIEncU256Instruction with region should have correct region")
		}
	} else {
		t.Error("ABIEncU256Instruction with region should have MemoryEffectOp")
	}
}

func TestEventSignatureInstructionEffects(t *testing.T) {
	eventSig := &EventSignatureInstruction{}
	effects := eventSig.GetEffects()

	if len(effects) != 1 {
		t.Errorf("EventSignatureInstruction should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("EventSignatureInstruction should have PureEffect")
	}
}

func TestRevertInstructionEffects(t *testing.T) {
	revert := &RevertInstruction{}
	effects := revert.GetEffects()

	if len(effects) != 1 {
		t.Errorf("RevertInstruction should have 1 effect, got %d", len(effects))
	}

	if storageEffect, ok := effects[0].(*StorageEffect); ok {
		if storageEffect.Type != "revert" {
			t.Error("RevertInstruction should have revert effect")
		}
		if storageEffect.Slot != -1 {
			t.Errorf("RevertInstruction should have slot -1, got %d", storageEffect.Slot)
		}
	} else {
		t.Error("RevertInstruction should have StorageEffect")
	}
}

func TestReturnTerminatorEffects(t *testing.T) {
	returnTerm := &ReturnTerminator{}
	effects := returnTerm.GetEffects()

	if len(effects) != 1 {
		t.Errorf("ReturnTerminator should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("ReturnTerminator should have PureEffect")
	}
}

func TestBranchTerminatorEffects(t *testing.T) {
	branchTerm := &BranchTerminator{}
	effects := branchTerm.GetEffects()

	if len(effects) != 1 {
		t.Errorf("BranchTerminator should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("BranchTerminator should have PureEffect")
	}
}

func TestJumpTerminatorEffects(t *testing.T) {
	jumpTerm := &JumpTerminator{}
	effects := jumpTerm.GetEffects()

	if len(effects) != 1 {
		t.Errorf("JumpTerminator should have 1 effect, got %d", len(effects))
	}

	if _, ok := effects[0].(*PureEffect); !ok {
		t.Error("JumpTerminator should have PureEffect")
	}
}
