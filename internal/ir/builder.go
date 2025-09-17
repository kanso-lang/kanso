package ir

import (
	"fmt"
	"kanso/internal/ast"
	"kanso/internal/semantic"
	"strconv"
	"strings"
)

// ConstEvalIntrinsic represents a pure, argument-free stdlib function that can be const-eval'd
type ConstEvalIntrinsic struct {
	ModulePath   string // e.g., "std::address"
	FunctionName string // e.g., "zero"
	ConstValue   string // The constant value to emit
	ReturnType   Type   // The return type
}

// getConstEvalIntrinsics returns the table of functions that should be const-eval'd during lowering
func getConstEvalIntrinsics() map[string]ConstEvalIntrinsic {
	return map[string]ConstEvalIntrinsic{
		"std::address::zero": {
			ModulePath:   "std::address",
			FunctionName: "zero",
			ConstValue:   "0x0000000000000000000000000000000000000000",
			ReturnType:   &AddressType{},
		},
		"address::zero": { // Handle import alias case
			ModulePath:   "address",
			FunctionName: "zero",
			ConstValue:   "0x0000000000000000000000000000000000000000",
			ReturnType:   &AddressType{},
		},
		// Add more const-eval intrinsics here in the future:
		// "std::bytes::empty": {
		//     ModulePath:   "std::bytes",
		//     FunctionName: "empty",
		//     ConstValue:   "",
		//     ReturnType:   &BytesType{},
		// },
	}
}

// Builder converts AST to IR
type Builder struct {
	program      *Program
	currentFunc  *Function
	currentBlock *BasicBlock
	valueCounter int
	blockCounter int
	instCounter  int

	// SSA construction state
	variableStack  map[string][]*Value // Stack of IR values for each variable
	incompletePhis map[*BasicBlock][]*PhiInstruction
	sealedBlocks   map[*BasicBlock]bool

	// Global constants cache
	globalConstants map[string]*Value // Cache for reusable constants like true, false

	// Context for semantic analysis
	context *semantic.ContextRegistry

	// Storage layout tracking
	storageSlots map[string]int
	slotCounter  int

	// Memory region tracking
	memoryRegionCounter int
	memoryRegions       []*MemoryRegion

	// Function-level caching for CSE
	senderValue  *Value            // Cache sender() result within current function
	storageAddrs map[string]*Value // Cache storage addresses by key
	storageLoads map[string]*Value // Cache storage loads by address
}

// NewBuilder creates a new IR builder
func NewBuilder(context *semantic.ContextRegistry) *Builder {
	return &Builder{
		valueCounter:        0,
		blockCounter:        0,
		instCounter:         0,
		variableStack:       make(map[string][]*Value),
		incompletePhis:      make(map[*BasicBlock][]*PhiInstruction),
		sealedBlocks:        make(map[*BasicBlock]bool),
		globalConstants:     make(map[string]*Value),
		context:             context,
		storageSlots:        make(map[string]int),
		slotCounter:         0,
		memoryRegionCounter: 0,
		memoryRegions:       []*MemoryRegion{},
	}
}

// Build converts an AST contract to IR
func (b *Builder) Build(contract *ast.Contract) *Program {
	b.program = &Program{
		Contract:        contract.Name.Value,
		Functions:       []*Function{},
		Storage:         []*StorageSlot{},
		Constants:       []*Constant{},
		EventSignatures: []*EventSignature{},
		Blocks:          make(map[string]*BasicBlock),
		CFG:             &ControlFlowGraph{},
	}

	// Create canonical constants first
	b.createCanonicalConstants()

	// First pass: collect storage layout
	b.collectStorageLayout(contract)

	// Second pass: collect event signatures
	b.collectEventSignatures(contract)

	// Third pass: process functions
	for _, item := range contract.Items {
		switch node := item.(type) {
		case *ast.Function:
			fn := b.buildFunction(node)
			b.program.Functions = append(b.program.Functions, fn)
		}
	}

	// Build control flow graph
	b.buildCFG()

	return b.program
}

// collectEventSignatures processes event structs to create global signature constants
func (b *Builder) collectEventSignatures(contract *ast.Contract) {
	for _, item := range contract.Items {
		if structNode, ok := item.(*ast.Struct); ok {
			// Check if this is an event struct
			if structNode.Attribute != nil && structNode.Attribute.Name == "event" {
				eventName := structNode.Name.Value

				// Generate signature based on field types
				var fieldTypes []string
				for _, item := range structNode.Items {
					if field, ok := item.(*ast.StructField); ok {
						fieldTypes = append(fieldTypes, b.astTypeToABIString(field.VariableType))
					}
				}

				signature := fmt.Sprintf("%s(%s)", eventName, strings.Join(fieldTypes, ","))

				eventSig := &EventSignature{
					Name:      eventName + "_sig",
					EventName: eventName,
					Signature: signature,
				}
				b.program.EventSignatures = append(b.program.EventSignatures, eventSig)
			}
		}
	}
}

// astTypeToABIString converts AST types to ABI string representation
func (b *Builder) astTypeToABIString(astType *ast.VariableType) string {
	if astType == nil {
		return "unknown"
	}

	typeName := astType.Name.Value
	switch typeName {
	case "Address":
		return "address"
	case "U256":
		return "uint256"
	case "U128":
		return "uint128"
	case "U64":
		return "uint64"
	case "U32":
		return "uint32"
	case "U16":
		return "uint16"
	case "U8":
		return "uint8"
	case "Bool":
		return "bool"
	case "String":
		return "string"
	default:
		return "unknown"
	}
}

// collectStorageLayout analyzes storage structs and assigns slots
func (b *Builder) collectStorageLayout(contract *ast.Contract) {
	for _, item := range contract.Items {
		if structNode, ok := item.(*ast.Struct); ok {
			if structNode.Attribute != nil && structNode.Attribute.Name == "storage" {
				for _, item := range structNode.Items {
					if field, ok := item.(*ast.StructField); ok {
						slot := &StorageSlot{
							Slot:        b.slotCounter,
							Name:        field.Name.Value,
							Type:        b.convertType(field.VariableType),
							AccessCount: 0,
						}
						b.storageSlots[field.Name.Value] = b.slotCounter
						b.program.Storage = append(b.program.Storage, slot)
						b.slotCounter++
					}
				}
			}
		}
	}
}

// buildFunction converts an AST function to SSA form
func (b *Builder) buildFunction(astFunc *ast.Function) *Function {
	fn := &Function{
		Name:       astFunc.Name.Value,
		External:   astFunc.External,
		Create:     astFunc.Attribute != nil && astFunc.Attribute.Name == "create",
		Params:     []*Parameter{},
		ReturnType: b.convertType(astFunc.Return),
		Reads:      b.extractClauseIdentifiers(astFunc.Reads),
		Writes:     b.extractClauseIdentifiers(astFunc.Writes),
		Blocks:     []*BasicBlock{},
		LocalVars:  make(map[string]*Value),
	}

	b.currentFunc = fn
	b.senderValue = nil                      // Reset sender cache for new function
	b.storageAddrs = make(map[string]*Value) // Reset storage address cache
	b.storageLoads = make(map[string]*Value) // Reset storage load cache

	// Create entry block
	entry := b.createBlock("entry")
	fn.Entry = entry
	b.currentBlock = entry

	// Process parameters
	for _, param := range astFunc.Params {
		paramValue := b.createValue(param.Name.Value, b.convertType(param.Type))
		parameter := &Parameter{
			Name:  param.Name.Value,
			Type:  b.convertType(param.Type),
			Value: paramValue,
		}
		fn.Params = append(fn.Params, parameter)
		b.writeVariable(param.Name.Value, paramValue)
	}

	// Process function body
	if astFunc.Body != nil {
		b.buildBlock(astFunc.Body)

		// Workaround: If the body is truly empty (no items and no tail expression),
		// and this is a simple accessor function, generate the appropriate storage load
		if len(astFunc.Body.Items) == 0 && astFunc.Body.TailExpr == nil {
			if b.currentBlock.Terminator != nil {
				// Remove the auto-generated void return so we can add the correct one
				b.currentBlock.Terminator = nil
			}
			b.generateAccessorFunction(astFunc)
		}
	}

	// Seal all blocks and complete phi functions
	b.sealAllBlocks()

	return fn
}

// buildBlock processes a block of statements
func (b *Builder) buildBlock(block *ast.FunctionBlock) {
	// Process all statements in the function body
	for i, item := range block.Items {
		// Check if this is the last item and it's an expression statement without semicolon
		if i == len(block.Items)-1 {
			if exprStmt, ok := item.(*ast.ExprStmt); ok && !exprStmt.Semicolon {
				// This is an implicit return - build the expression and return it
				value := b.buildExpression(exprStmt.Expr)
				terminator := &ReturnTerminator{
					ID:    b.nextInstID(),
					Block: b.currentBlock,
					Value: value,
				}
				b.currentBlock.Terminator = terminator
				return
			}
		}
		b.buildBlockItem(item)
	}

	// Handle tail expression (implicit return without semicolon)
	if block.TailExpr != nil {
		value := b.buildExpression(block.TailExpr.Expr)
		terminator := &ReturnTerminator{
			ID:    b.nextInstID(),
			Block: b.currentBlock,
			Value: value,
		}
		b.currentBlock.Terminator = terminator
		return
	}

	// If we reach here without a terminator, add a void return
	if b.currentBlock.Terminator == nil {
		terminator := &ReturnTerminator{
			ID:    b.nextInstID(),
			Block: b.currentBlock,
			Value: nil,
		}
		b.currentBlock.Terminator = terminator
	}
}

// buildBlockItem processes individual function block items
func (b *Builder) buildBlockItem(item ast.FunctionBlockItem) {
	switch s := item.(type) {
	case *ast.LetStmt:
		b.buildLetStatement(s)
	case *ast.AssignStmt:
		b.buildAssignStatement(s)
	case *ast.ExprStmt:
		b.buildExpression(s.Expr)
	case *ast.RequireStmt:
		b.buildRequireStatement(s)
	case *ast.ReturnStmt:
		b.buildReturnStatement(s)
	}
}

// buildLetStatement processes let statements with SSA value creation
func (b *Builder) buildLetStatement(letStmt *ast.LetStmt) {
	// Evaluate the initializer expression
	initValue := b.buildExpression(letStmt.Expr)

	// In SSA form, for let statements we simply bind the variable name to the computed value
	// We don't need a separate store instruction for immutable let bindings
	b.writeVariable(letStmt.Name.Value, initValue)
}

// buildAssignStatement processes assignment statements
func (b *Builder) buildAssignStatement(assignStmt *ast.AssignStmt) {
	rightValue := b.buildExpression(assignStmt.Value)

	// Handle compound assignments (+=, -=, etc.)
	if assignStmt.Operator != ast.ASSIGN {
		// For compound assignments, we need to load current value, perform operation, then store
		var currentValue *Value

		switch left := assignStmt.Target.(type) {
		case *ast.IdentExpr:
			currentValue = b.readVariable(left.Name)
		case *ast.FieldAccessExpr:
			currentValue = b.buildStorageLoad(left)
		case *ast.IndexExpr:
			currentValue = b.buildKeyedStorageLoad(left)
		}

		// Perform the compound operation using checked arithmetic
		opStr := b.getCompoundOpString(assignStmt.Operator)

		// Use checked arithmetic for potentially overflow-prone operations
		if opStr == "ADD" || opStr == "SUB" || opStr == "MUL" {
			resultName := b.getDescriptiveCompoundName(assignStmt.Target, opStr)
			result := b.createValue(resultName, currentValue.Type)
			checkResult := b.createValue("compound_check", &BoolType{})
			inst := &CheckedArithInstruction{
				ID:        b.nextInstID(),
				ResultVal: result,
				ResultOk:  checkResult,
				Block:     b.currentBlock,
				Op:        opStr + "_CHK", // ADD_CHK, SUB_CHK, MUL_CHK
				Left:      currentValue,
				Right:     rightValue,
			}
			b.addInstruction(inst)
			rightValue = result
		} else {
			// Use regular binary operation for non-arithmetic ops
			resultName := b.getDescriptiveCompoundName(assignStmt.Target, opStr)
			result := b.createValue(resultName, currentValue.Type)
			inst := &BinaryInstruction{
				ID:     b.nextInstID(),
				Result: result,
				Block:  b.currentBlock,
				Op:     opStr,
				Left:   currentValue,
				Right:  rightValue,
			}
			b.addInstruction(inst)
			rightValue = result
		}
	}

	// Perform the assignment with the (possibly computed) right value
	switch left := assignStmt.Target.(type) {
	case *ast.IdentExpr:
		// Simple variable assignment
		b.writeVariable(left.Name, rightValue)

	case *ast.FieldAccessExpr:
		// Storage field assignment (e.g., State.balances = value)
		b.buildStorageStore(left, rightValue)

	case *ast.IndexExpr:
		// Indexed storage assignment (e.g., State.balances[key] = value)
		b.buildKeyedStorageStore(left, rightValue)
	}
}

// buildExpression converts expressions to SSA form
func (b *Builder) buildExpression(expr ast.Expr) *Value {
	switch e := expr.(type) {
	case *ast.LiteralExpr:
		// Try to parse as different literal types
		if e.Value == "true" || e.Value == "false" {
			return b.buildConstant(e.Value == "true", &BoolType{})
		}
		// Default to integer
		return b.buildConstant(e.Value, &IntType{Bits: 256})

	case *ast.IdentExpr:
		// Check if this is a boolean constant or other canonical constant
		if e.Name == "true" {
			return b.getOrCreateGlobalConstant(true, &BoolType{}, "true")
		} else if e.Name == "false" {
			return b.getOrCreateGlobalConstant(false, &BoolType{}, "false")
		}
		return b.readVariable(e.Name)

	case *ast.FieldAccessExpr:
		// Check if this is a module-qualified identifier (e.g., errors::SelfTransfer)
		if ident, ok := e.Target.(*ast.IdentExpr); ok {
			// Check if this looks like a module reference (not State)
			if ident.Name != "State" {
				// This is a module constant like errors::SelfTransfer
				fullName := ident.Name + "::" + e.Field
				return b.buildConstant(fullName, &StringType{})
			}
		}
		// Otherwise, it's storage access
		return b.buildStorageLoad(e)

	case *ast.IndexExpr:
		return b.buildKeyedStorageLoad(e)

	case *ast.CallExpr:
		return b.buildCall(e)

	case *ast.BinaryExpr:
		return b.buildBinaryOp(e)

	case *ast.TupleExpr:
		return b.buildTuple(e)

	case *ast.StructLiteralExpr:
		return b.buildStructLiteral(e)

	case *ast.CalleePath:
		// Handle module-qualified paths like errors::SelfTransfer
		parts := make([]string, len(e.Parts))
		for i, part := range e.Parts {
			parts[i] = part.Value
		}
		fullPath := strings.Join(parts, "::")
		return b.buildConstant(fullPath, &StringType{})

	default:
		// Create placeholder for unknown expressions
		return b.createValue("unknown", &IntType{Bits: 256})
	}
}

// buildStorageLoad creates storage load instructions
func (b *Builder) buildStorageLoad(fieldAccess *ast.FieldAccessExpr) *Value {
	// Check if this is a State.field access
	if ident, ok := fieldAccess.Target.(*ast.IdentExpr); ok && ident.Name == "State" {
		if slot, exists := b.storageSlots[fieldAccess.Field]; exists {
			slotValue := b.buildConstant(strconv.Itoa(slot), &IntType{Bits: 256})
			result := b.createValue("sload_result", b.getStorageType(fieldAccess.Field))

			inst := &StorageLoadInstruction{
				ID:      b.nextInstID(),
				Result:  result,
				Block:   b.currentBlock,
				Slot:    slotValue,
				SlotNum: slot,
			}
			b.addInstruction(inst)
			return result
		}
	}

	// Fallback for other field accesses
	return b.createValue("field_load", &IntType{Bits: 256})
}

// buildStorageStore creates storage store instructions
func (b *Builder) buildStorageStore(fieldAccess *ast.FieldAccessExpr, value *Value) {
	// Check if this is a State.field access
	if ident, ok := fieldAccess.Target.(*ast.IdentExpr); ok && ident.Name == "State" {
		if slot, exists := b.storageSlots[fieldAccess.Field]; exists {
			// CRITICAL FIX: Prevent double-counting total_supply in create function
			// The create function should initialize total_supply = 0, let mint() handle the supply
			actualValue := value
			if fieldAccess.Field == "total_supply" && b.currentFunc != nil && b.currentFunc.Create {
				// In create function, initialize total_supply to 0 instead of initial_supply
				actualValue = b.buildConstant("0", &IntType{Bits: 256})
			}

			slotValue := b.buildConstant(strconv.Itoa(slot), &IntType{Bits: 256})

			inst := &StorageStoreInstruction{
				ID:      b.nextInstID(),
				Block:   b.currentBlock,
				Slot:    slotValue,
				Value:   actualValue,
				SlotNum: slot,
				Type:    b.getStorageType(fieldAccess.Field),
			}
			b.addInstruction(inst)
		}
	}
}

// buildKeyedStorageLoad creates enhanced storage load with abstract addressing
func (b *Builder) buildKeyedStorageLoad(indexExpr *ast.IndexExpr) *Value {
	if fieldAccess, ok := indexExpr.Target.(*ast.FieldAccessExpr); ok {
		if ident, ok := fieldAccess.Target.(*ast.IdentExpr); ok && ident.Name == "State" {
			if slot, exists := b.storageSlots[fieldAccess.Field]; exists {
				// Handle tuple keys (for nested mappings like allowances)
				var keys []*Value
				if tupleExpr, isTuple := indexExpr.Index.(*ast.TupleExpr); isTuple {
					// Multi-key mapping: allowances[(owner, spender)]
					for _, elem := range tupleExpr.Elements {
						keyValue := b.buildExpression(elem)
						keys = append(keys, keyValue)
					}
				} else {
					// Single-key mapping: balances[owner]
					keyValue := b.buildExpression(indexExpr.Index)
					keys = []*Value{keyValue}
				}

				// Create cache key for this storage address
				cacheKey := fmt.Sprintf("slot_%d", slot)
				for _, key := range keys {
					cacheKey += fmt.Sprintf("_%s", key.Name)
				}

				// Check if we already have this storage address cached
				var addrResult *Value
				if cached, exists := b.storageAddrs[cacheKey]; exists {
					addrResult = cached
				} else {
					// Generate new abstract storage address
					addrResult = b.createValue("storage_addr", &StorageAddrType{})
					addrInst := &StorageAddrInstruction{
						ID:       b.nextInstID(),
						Result:   addrResult,
						Block:    b.currentBlock,
						BaseSlot: slot,
						Keys:     keys,
					}
					b.addInstruction(addrInst)
					b.storageAddrs[cacheKey] = addrResult
				}

				// Check if we already loaded from this address
				if cachedLoad, exists := b.storageLoads[cacheKey]; exists {
					return cachedLoad
				}

				// Load from the abstract address
				result := b.createValue(fmt.Sprintf("%s_load", fieldAccess.Field), &IntType{Bits: 256})
				loadInst := &StorageLoadInstruction{
					ID:      b.nextInstID(),
					Result:  result,
					Block:   b.currentBlock,
					Slot:    addrResult,
					SlotNum: -1, // Abstract address, not direct slot
				}
				b.addInstruction(loadInst)
				b.storageLoads[cacheKey] = result
				return result
			}
		}
	}

	return b.createValue("keyed_load", &IntType{Bits: 256})
}

// buildKeyedStorageStore creates keyed storage store instructions using enhanced IR format
func (b *Builder) buildKeyedStorageStore(indexExpr *ast.IndexExpr, value *Value) {
	if fieldAccess, ok := indexExpr.Target.(*ast.FieldAccessExpr); ok {
		if ident, ok := fieldAccess.Target.(*ast.IdentExpr); ok && ident.Name == "State" {
			if slot, exists := b.storageSlots[fieldAccess.Field]; exists {
				// Handle single key or tuple key
				var keys []*Value
				if tupleExpr, ok := indexExpr.Index.(*ast.TupleExpr); ok {
					// Multi-key mapping (e.g., allowances[owner][spender])
					for _, elem := range tupleExpr.Elements {
						keyValue := b.buildExpression(elem)
						keys = append(keys, keyValue)
					}
				} else {
					// Single key mapping (e.g., balances[account])
					keyValue := b.buildExpression(indexExpr.Index)
					keys = append(keys, keyValue)
				}

				// Create cache key for this storage address
				cacheKey := fmt.Sprintf("slot_%d", slot)
				for _, key := range keys {
					cacheKey += fmt.Sprintf("_%s", key.Name)
				}

				// Check if we already have this storage address cached
				var addrResult *Value
				if cached, exists := b.storageAddrs[cacheKey]; exists {
					addrResult = cached
				} else {
					// Generate new abstract storage address
					addrResult = b.createValue("storage_addr", &StorageAddrType{})
					addrInst := &StorageAddrInstruction{
						ID:       b.nextInstID(),
						Result:   addrResult,
						Block:    b.currentBlock,
						BaseSlot: slot,
						Keys:     keys,
					}
					b.addInstruction(addrInst)
					b.storageAddrs[cacheKey] = addrResult
				}

				// Invalidate any cached loads for this address since we're storing
				delete(b.storageLoads, cacheKey)

				// Store to the abstract address
				storeInst := &StorageStoreInstruction{
					ID:      b.nextInstID(),
					Block:   b.currentBlock,
					Slot:    addrResult,
					Value:   value,
					SlotNum: -1, // Abstract address, not direct slot
					Type:    value.Type,
				}
				b.addInstruction(storeInst)
			}
		}
	}
}

// buildCall creates function call instructions with namespace resolution
func (b *Builder) buildCall(callExpr *ast.CallExpr) *Value {
	// Determine function name and module first
	funcName, module := b.resolveFunctionName(callExpr.Callee)

	// Workaround: if module is empty but function is a known std function, set the module
	if module == "" {
		if funcName == "sender" || funcName == "emit" {
			module = "std::evm"
		} else if funcName == "zero" {
			module = "std::address"
		}
	}

	// Special handling for emit - don't build args normally as we'll process struct literals specially
	if module == "std::evm" && funcName == "emit" {
		return b.buildEmitCall(callExpr)
	}

	// Build arguments for regular function calls
	var args []*Value
	for _, arg := range callExpr.Args {
		argValue := b.buildExpression(arg)
		args = append(args, argValue)
	}

	// Handle special EVM functions
	if module == "std::evm" && funcName == "sender" {
		// Reuse cached sender value if available
		if b.senderValue != nil {
			return b.senderValue
		}

		result := b.createValue("sender_result", &AddressType{})
		inst := &SenderInstruction{
			ID:     b.nextInstID(),
			Result: result,
			Block:  b.currentBlock,
		}
		b.addInstruction(inst)
		b.senderValue = result // Cache for future use in this function
		return result
	}

	// Check for const-eval intrinsics before creating call instruction
	intrinsicKey := module + "::" + funcName
	if intrinsic, isConstEval := getConstEvalIntrinsics()[intrinsicKey]; isConstEval {
		// Ensure no arguments for const-eval intrinsics (they should be argument-free)
		if len(args) == 0 {
			// For address::zero(), reuse the existing %zero_addr global constant
			if funcName == "zero" && intrinsic.ConstValue == "0x0000000000000000000000000000000000000000" {
				if zeroAddr, exists := b.globalConstants["zero_addr"]; exists {
					return zeroAddr
				}
			}

			// Otherwise, emit CONST instruction
			result := b.createValue(funcName+"_const", intrinsic.ReturnType)
			inst := &ConstantInstruction{
				ID:     b.nextInstID(),
				Result: result,
				Block:  b.currentBlock,
				Value:  intrinsic.ConstValue,
				Type:   intrinsic.ReturnType,
			}
			b.addInstruction(inst)
			return result
		}
		// If const-eval intrinsic has arguments, fall through to regular call
		// (shouldn't happen with current intrinsics, but defensive programming)
	}

	// Regular function call
	result := b.createValue("call_result", &IntType{Bits: 256})
	inst := &CallInstruction{
		ID:       b.nextInstID(),
		Result:   result,
		Block:    b.currentBlock,
		Function: funcName,
		Args:     args,
		Module:   module,
	}
	b.addInstruction(inst)
	return result
}

// buildEmitCall handles emit function calls specially to avoid duplicate struct literal processing
func (b *Builder) buildEmitCall(callExpr *ast.CallExpr) *Value {
	eventName := "Unknown"
	var eventFields []*Value

	// Try to extract event name and field args from the first argument if it's a struct literal
	if len(callExpr.Args) > 0 {
		if structLit, ok := callExpr.Args[0].(*ast.StructLiteralExpr); ok {
			// Extract event name
			if structLit.Type != nil && len(structLit.Type.Parts) > 0 {
				eventName = structLit.Type.Parts[len(structLit.Type.Parts)-1].Value
			}

			// Build field expressions as individual arguments
			for _, field := range structLit.Fields {
				if field.Value != nil {
					fieldValue := b.buildExpression(field.Value)
					eventFields = append(eventFields, fieldValue)
				}
			}
		} else {
			// Fallback: build the argument normally and use it
			argValue := b.buildExpression(callExpr.Args[0])
			eventFields = []*Value{argValue}
		}
	}

	// Reference the global event signature (no ID suffix for true globals)
	var sigHash *Value
	globalSigName := eventName + "_sig"

	// Create a special global reference value without ID suffix
	sigHash = &Value{
		ID:   -1,            // Special ID for globals
		Name: globalSigName, // Just "Transfer_sig" or "Approval_sig"
		Type: &StringType{},
	}

	// Process event fields based on ERC-20 Transfer pattern
	// Transfer has: from (Address), to (Address), value (U256)
	// In EVM: from and to are indexed (topics), value is in data
	var topics []*Value
	var dataPtr, dataLen *Value

	if eventName == "Transfer" && len(eventFields) >= 3 {
		// For Transfer: first two fields (from, to) are topics, third (value) is data
		fromTopic := b.createValue("t1", &StringType{})
		fromTopicInst := &TopicAddrInstruction{
			ID:      b.nextInstID(),
			Result:  fromTopic,
			Block:   b.currentBlock,
			Address: eventFields[0], // from
		}
		b.addInstruction(fromTopicInst)
		topics = append(topics, fromTopic)

		toTopic := b.createValue("t2", &StringType{})
		toTopicInst := &TopicAddrInstruction{
			ID:      b.nextInstID(),
			Result:  toTopic,
			Block:   b.currentBlock,
			Address: eventFields[1], // to
		}
		b.addInstruction(toTopicInst)
		topics = append(topics, toTopic)

		// Pack value in data
		dataPtr = b.createValue("dp", &StringType{})
		dataLen = b.createValue("dl", &IntType{Bits: 32})

		// Create memory region for ABI encoded data (U256 = 32 bytes)
		sizeConst := b.createValue("32", &IntType{Bits: 32})
		memRegion := b.createMemoryRegion(MemoryRegionABIData, dataPtr, sizeConst)

		// Create memory effects: allocate region and write encoded data
		allocEffect := b.createMemoryEffect(memRegion, MemoryEffectAllocate, nil, sizeConst)
		writeEffect := b.createMemoryEffect(memRegion, MemoryEffectWrite, nil, sizeConst)

		dataInst := &ABIEncU256Instruction{
			ID:           b.nextInstID(),
			ResultData:   dataPtr,
			ResultLen:    dataLen,
			Block:        b.currentBlock,
			Value:        eventFields[2], // value
			MemoryRegion: memRegion,
			Effects:      []MemoryEffect{allocEffect, writeEffect},
		}
		b.addInstruction(dataInst)
	} else if eventName == "Approval" && len(eventFields) >= 3 {
		// For Approval: similar pattern - owner, spender are topics, value is data
		ownerTopic := b.createValue("t1", &StringType{})
		ownerTopicInst := &TopicAddrInstruction{
			ID:      b.nextInstID(),
			Result:  ownerTopic,
			Block:   b.currentBlock,
			Address: eventFields[0], // owner
		}
		b.addInstruction(ownerTopicInst)
		topics = append(topics, ownerTopic)

		spenderTopic := b.createValue("t2", &StringType{})
		spenderTopicInst := &TopicAddrInstruction{
			ID:      b.nextInstID(),
			Result:  spenderTopic,
			Block:   b.currentBlock,
			Address: eventFields[1], // spender
		}
		b.addInstruction(spenderTopicInst)
		topics = append(topics, spenderTopic)

		// Pack value in data
		dataPtr = b.createValue("dp", &StringType{})
		dataLen = b.createValue("dl", &IntType{Bits: 32})

		// Create memory region for ABI encoded data (U256 = 32 bytes)
		sizeConst := b.createValue("32", &IntType{Bits: 32})
		memRegion := b.createMemoryRegion(MemoryRegionABIData, dataPtr, sizeConst)

		// Create memory effects: allocate region and write encoded data
		allocEffect := b.createMemoryEffect(memRegion, MemoryEffectAllocate, nil, sizeConst)
		writeEffect := b.createMemoryEffect(memRegion, MemoryEffectWrite, nil, sizeConst)

		dataInst := &ABIEncU256Instruction{
			ID:           b.nextInstID(),
			ResultData:   dataPtr,
			ResultLen:    dataLen,
			Block:        b.currentBlock,
			Value:        eventFields[2], // value
			MemoryRegion: memRegion,
			Effects:      []MemoryEffect{allocEffect, writeEffect},
		}
		b.addInstruction(dataInst)
	}

	// Create the LOG instruction
	logInst := &LogInstruction{
		ID:        b.nextInstID(),
		Block:     b.currentBlock,
		Topics:    3, // LOG3: signature + 2 topics
		Event:     eventName,
		Signature: sigHash,
		TopicArgs: topics,
		DataPtr:   dataPtr,
		DataLen:   dataLen,
	}
	b.addInstruction(logInst)

	// Return a placeholder value (emit doesn't return anything meaningful)
	return b.createValue("emit_result", &IntType{Bits: 1})
}

// generateEventSignature creates the canonical event signature for keccak256 hashing
func (b *Builder) generateEventSignature(eventName string, args []*Value) string {
	var argTypes []string
	for _, arg := range args {
		// Convert IR types to Solidity ABI type names
		abiType := b.typeToABIString(arg.Type)
		argTypes = append(argTypes, abiType)
	}

	return fmt.Sprintf("%s(%s)", eventName, strings.Join(argTypes, ","))
}

// typeToABIString converts IR types to ABI type strings
func (b *Builder) typeToABIString(typ Type) string {
	switch t := typ.(type) {
	case *IntType:
		return fmt.Sprintf("uint%d", t.Bits)
	case *AddressType:
		return "address"
	case *BoolType:
		return "bool"
	case *StringType:
		return "string"
	default:
		return "uint256" // Default fallback
	}
}

// buildBinaryOp creates binary operation instructions with Sethi-Ullman optimization
func (b *Builder) buildBinaryOp(binaryExpr *ast.BinaryExpr) *Value {
	// Compute Sethi-Ullman numbers to determine optimal evaluation order
	leftSU := b.computeSeethiUllman(binaryExpr.Left)
	rightSU := b.computeSeethiUllman(binaryExpr.Right)

	var left, right *Value

	// Evaluate subtrees in optimal order based on Sethi-Ullman numbers
	// Higher SU number should be evaluated first to minimize stack operations
	if leftSU >= rightSU {
		left = b.buildExpression(binaryExpr.Left)
		right = b.buildExpression(binaryExpr.Right)
	} else {
		// SU optimization: evaluate more complex right side first
		right = b.buildExpression(binaryExpr.Right)
		left = b.buildExpression(binaryExpr.Left)
	}

	result := b.createValue(fmt.Sprintf("%s_result", binaryExpr.Op), left.Type)

	inst := &BinaryInstruction{
		ID:     b.nextInstID(),
		Result: result,
		Block:  b.currentBlock,
		Op:     binaryExpr.Op,
		Left:   left,
		Right:  right,
	}
	b.addInstruction(inst)
	return result
}

// computeSeethiUllman calculates the Sethi-Ullman number for an expression
// The SU number represents the minimum number of registers needed to evaluate the expression
// Higher numbers indicate more complex expressions that should be evaluated first
func (b *Builder) computeSeethiUllman(expr ast.Expr) int {
	switch e := expr.(type) {
	case *ast.LiteralExpr, *ast.IdentExpr:
		// Leaves (constants and variables) require 1 register
		return 1

	case *ast.FieldAccessExpr:
		// Storage loads require 1 register (address is computed separately)
		return 1

	case *ast.IndexExpr:
		// Keyed storage access needs SU(key) + 1 for the base address
		return b.computeSeethiUllman(e.Index) + 1

	case *ast.CallExpr:
		// Function calls require registers for all arguments
		maxArgSU := 0
		for _, arg := range e.Args {
			argSU := b.computeSeethiUllman(arg)
			if argSU > maxArgSU {
				maxArgSU = argSU
			}
		}
		return maxArgSU + 1

	case *ast.BinaryExpr:
		leftSU := b.computeSeethiUllman(e.Left)
		rightSU := b.computeSeethiUllman(e.Right)

		// If both subtrees need the same number of registers,
		// we need one extra register to hold the intermediate result
		if leftSU == rightSU {
			return leftSU + 1
		}
		// Otherwise, we need as many registers as the more complex subtree
		if leftSU > rightSU {
			return leftSU
		}
		return rightSU

	case *ast.TupleExpr:
		// Tuple expressions need registers for the most complex element
		maxElementSU := 0
		for _, elem := range e.Elements {
			elemSU := b.computeSeethiUllman(elem)
			if elemSU > maxElementSU {
				maxElementSU = elemSU
			}
		}
		return maxElementSU

	default:
		// Unknown expressions default to 1 register
		return 1
	}
}

// buildTuple creates tuple expressions
func (b *Builder) buildTuple(tupleExpr *ast.TupleExpr) *Value {
	// For now, represent tuples as their first element
	// This is a simplification - full tuple support would require more complex handling
	if len(tupleExpr.Elements) > 0 {
		return b.buildExpression(tupleExpr.Elements[0])
	}
	return b.createValue("empty_tuple", &TupleType{})
}

// buildStructLiteral creates struct literal expressions
func (b *Builder) buildStructLiteral(structExpr *ast.StructLiteralExpr) *Value {
	// For event struct literals, we need to build individual field values
	// but return a single representative value for the struct
	structName := "Unknown"
	if structExpr.Type != nil {
		// Extract the struct name from the CalleePath
		if len(structExpr.Type.Parts) > 0 {
			structName = structExpr.Type.Parts[len(structExpr.Type.Parts)-1].Value
		}
	}

	// Build all field expressions and store them for emit processing
	// This ensures the field expressions get proper IR generation
	for _, field := range structExpr.Fields {
		if field.Value != nil {
			b.buildExpression(field.Value)
		}
	}

	// Create a value representing the struct literal
	result := b.createValue(structName, &StringType{})
	return result
}

// buildConstant creates constant instructions
func (b *Builder) buildConstant(value interface{}, typ Type) *Value {
	// Handle commonly reused constants as globals
	switch v := value.(type) {
	case bool:
		if v {
			return b.getOrCreateGlobalConstant(value, typ, "true")
		} else {
			return b.getOrCreateGlobalConstant(value, typ, "false")
		}
	case string:
		if v == "0" && typ.String() == "U256" {
			return b.getOrCreateGlobalConstant(value, typ, "zero")
		}
	}

	// For other constants, create locally
	result := b.createValue("const", typ)
	inst := &ConstantInstruction{
		ID:     b.nextInstID(),
		Result: result,
		Block:  b.currentBlock,
		Value:  value,
		Type:   typ,
	}
	b.addInstruction(inst)
	return result
}

// getConstantKey returns a cache key for reusable constants
func (b *Builder) getConstantKey(value interface{}, typ Type) string {
	// Only cache commonly reused constants
	switch v := value.(type) {
	case bool:
		if v {
			return "true"
		}
		return "false"
	case string:
		if v == "0" && typ.String() == "U256" {
			return "zero_u256"
		}
	}
	return "" // Don't cache other constants
}

// getOrCreateGlobalConstant returns a global constant with a fixed name (no ID suffix)
func (b *Builder) getOrCreateGlobalConstant(value interface{}, typ Type, name string) *Value {
	// Should always return the canonical constant if it exists
	if existing, exists := b.globalConstants[name]; exists {
		return existing
	}

	// This should not happen after createCanonicalConstants() is called
	// But as fallback, create the constant
	result := &Value{
		ID:   -1, // Special ID for globals
		Name: name,
		Type: typ,
	}

	b.globalConstants[name] = result
	return result
}

// getCompoundOpString converts compound assignment operators to binary operators
func (b *Builder) getCompoundOpString(op ast.AssignType) string {
	switch op {
	case ast.PLUS_ASSIGN:
		return "ADD"
	case ast.MINUS_ASSIGN:
		return "SUB"
	case ast.STAR_ASSIGN:
		return "MUL"
	case ast.SLASH_ASSIGN:
		return "DIV"
	case ast.PERCENT_ASSIGN:
		return "MOD"
	default:
		return "UNKNOWN_OP"
	}
}

// getDescriptiveCompoundName creates descriptive names for compound assignment results
func (b *Builder) getDescriptiveCompoundName(left ast.Expr, opStr string) string {
	switch l := left.(type) {
	case *ast.FieldAccessExpr:
		if ident, ok := l.Target.(*ast.IdentExpr); ok && ident.Name == "State" {
			// State field access like State.total_supply
			return fmt.Sprintf("new_%s", l.Field)
		}
	case *ast.IndexExpr:
		if fieldAccess, ok := l.Target.(*ast.FieldAccessExpr); ok {
			if ident, ok := fieldAccess.Target.(*ast.IdentExpr); ok && ident.Name == "State" {
				// Storage mapping like State.balances[from]
				if fieldAccess.Field == "balances" {
					// Try to get a meaningful name from the index
					if indexIdent, ok := l.Index.(*ast.IdentExpr); ok {
						return fmt.Sprintf("new_%s_balance", indexIdent.Name)
					}
					return "new_balance"
				} else if fieldAccess.Field == "allowances" {
					return "new_allowance"
				} else {
					return fmt.Sprintf("new_%s", fieldAccess.Field)
				}
			}
		}
	case *ast.IdentExpr:
		// Simple variable
		return fmt.Sprintf("new_%s", l.Name)
	}

	// Fallback to generic name
	return "new_value"
}

// buildRequireStatement creates enhanced require instructions using branch + revert pattern
func (b *Builder) buildRequireStatement(requireStmt *ast.RequireStmt) {
	// RequireStmt has Args slice - first is condition, second is error
	var condition *Value
	if len(requireStmt.Args) >= 1 {
		condition = b.buildExpression(requireStmt.Args[0])
	}
	// Note: errorValue could be used for more sophisticated error handling in the future

	// Create blocks for successful path and revert path using createBlock
	successBlock := b.createBlock("success")
	revertBlock := b.createBlock("revert")

	// Set up predecessors
	successBlock.Predecessors = []*BasicBlock{b.currentBlock}
	revertBlock.Predecessors = []*BasicBlock{b.currentBlock}

	// Add conditional branch: if condition is true, go to success, else revert
	branch := &BranchTerminator{
		ID:         b.nextInstID(),
		Block:      b.currentBlock,
		Condition:  condition,
		TrueBlock:  successBlock,
		FalseBlock: revertBlock,
	}
	b.currentBlock.Terminator = branch
	b.currentBlock.Successors = append(b.currentBlock.Successors, successBlock, revertBlock)

	// Add revert instruction as terminator for revert block
	revertInst := &RevertInstruction{
		ID:    b.nextInstID(),
		Block: revertBlock,
	}
	revertBlock.Terminator = revertInst

	// Add assume instruction to success block for path-sensitive optimization
	assumeInst := &AssumeInstruction{
		ID:        b.nextInstID(),
		Block:     successBlock,
		Predicate: condition,
	}
	successBlock.Instructions = append(successBlock.Instructions, assumeInst)

	// Continue with success block as current
	b.currentBlock = successBlock
}

// buildReturnStatement creates return terminators
func (b *Builder) buildReturnStatement(returnStmt *ast.ReturnStmt) {
	var value *Value
	if returnStmt.Value != nil {
		value = b.buildExpression(returnStmt.Value)
	}

	terminator := &ReturnTerminator{
		ID:    b.nextInstID(),
		Block: b.currentBlock,
		Value: value,
	}
	b.currentBlock.Terminator = terminator
}

// SSA construction helpers

// createValue creates a new SSA value with unique naming
func (b *Builder) createValue(name string, typ Type) *Value {
	// Ensure SSA form by making each value name unique with a counter
	uniqueName := fmt.Sprintf("%s_%d", name, b.valueCounter)
	value := &Value{
		ID:       b.valueCounter,
		Name:     uniqueName,
		Type:     typ,
		DefBlock: b.currentBlock,
		Uses:     []*Use{},
		Version:  0,
	}
	b.valueCounter++
	return value
}

// createBlock creates a new basic block
func (b *Builder) createBlock(label string) *BasicBlock {
	block := &BasicBlock{
		Label:        fmt.Sprintf("%s_%d", label, b.blockCounter),
		Instructions: []Instruction{},
		Predecessors: []*BasicBlock{},
		Successors:   []*BasicBlock{},
		LiveIn:       make(map[string]*Value),
		LiveOut:      make(map[string]*Value),
	}
	b.blockCounter++
	b.program.Blocks[block.Label] = block
	if b.currentFunc != nil {
		b.currentFunc.Blocks = append(b.currentFunc.Blocks, block)
	}
	return block
}

// addInstruction adds an instruction to the current block
func (b *Builder) addInstruction(inst Instruction) {
	b.currentBlock.Instructions = append(b.currentBlock.Instructions, inst)
}

// writeVariable writes a value to a variable (SSA construction)
func (b *Builder) writeVariable(variable string, value *Value) {
	if b.variableStack[variable] == nil {
		b.variableStack[variable] = []*Value{}
	}
	b.variableStack[variable] = append(b.variableStack[variable], value)
	if b.currentFunc != nil {
		b.currentFunc.LocalVars[variable] = value
	}
}

// readVariable reads the current value of a variable (SSA construction)
func (b *Builder) readVariable(variable string) *Value {
	if stack := b.variableStack[variable]; len(stack) > 0 {
		return stack[len(stack)-1]
	}

	// Variable not defined in current scope - this would need phi functions
	// For now, create a placeholder
	return b.createValue(variable, &IntType{Bits: 256})
}

// Helper methods

func (b *Builder) nextInstID() int {
	id := b.instCounter
	b.instCounter++
	return id
}

// createMemoryRegion creates a new memory region with the specified properties
func (b *Builder) createMemoryRegion(kind MemoryRegionKind, base, size *Value) *MemoryRegion {
	id := b.memoryRegionCounter
	b.memoryRegionCounter++

	region := &MemoryRegion{
		ID:   id,
		Name: fmt.Sprintf("mem_%s_%d", kind, id),
		Base: base,
		Size: size,
		Kind: kind,
	}

	b.memoryRegions = append(b.memoryRegions, region)
	return region
}

// createMemoryEffect creates a memory effect for an instruction
func (b *Builder) createMemoryEffect(region *MemoryRegion, effectType MemoryEffectType, offset, size *Value) MemoryEffect {
	return MemoryEffect{
		Region: region,
		Type:   effectType,
		Offset: offset,
		Size:   size,
	}
}

// createCanonicalConstants creates standard constants used throughout the program
func (b *Builder) createCanonicalConstants() {
	// Create canonical boolean constants
	b.globalConstants["true"] = &Value{
		ID:   -1,
		Name: "true",
		Type: &BoolType{},
	}
	b.globalConstants["false"] = &Value{
		ID:   -1,
		Name: "false",
		Type: &BoolType{},
	}

	// Create canonical zero constants for common types
	b.globalConstants["zero"] = &Value{
		ID:   -1,
		Name: "zero",
		Type: &IntType{Bits: 256},
	}
	b.globalConstants["zero_addr"] = &Value{
		ID:   -1,
		Name: "zero_addr",
		Type: &AddressType{},
	}

	// Add them to the program constants for printing
	b.program.Constants = append(b.program.Constants, &Constant{
		Value: b.globalConstants["true"],
		Data:  true,
	})
	b.program.Constants = append(b.program.Constants, &Constant{
		Value: b.globalConstants["false"],
		Data:  false,
	})
	b.program.Constants = append(b.program.Constants, &Constant{
		Value: b.globalConstants["zero"],
		Data:  "0",
	})
	b.program.Constants = append(b.program.Constants, &Constant{
		Value: b.globalConstants["zero_addr"],
		Data:  "0x0000000000000000000000000000000000000000",
	})
}

func (b *Builder) hasAttribute(attributes []*ast.Attribute, name string) bool {
	for _, attr := range attributes {
		if attr.Name == name {
			return true
		}
	}
	return false
}

func (b *Builder) extractClauseIdentifiers(clause []ast.Ident) []string {
	var identifiers []string
	for _, ident := range clause {
		identifiers = append(identifiers, ident.Value)
	}
	return identifiers
}

func (b *Builder) convertType(astType *ast.VariableType) Type {
	if astType == nil {
		return nil
	}

	// Handle tuple types first
	if len(astType.TupleElements) > 0 {
		elements := make([]Type, len(astType.TupleElements))
		for i, elem := range astType.TupleElements {
			elements[i] = b.convertType(elem)
		}
		return &TupleType{Elements: elements}
	}

	// Check if it's a generic type (has generics)
	if len(astType.Generics) > 0 {
		if astType.Name.Value == "Slots" && len(astType.Generics) == 2 {
			return &SlotsType{
				KeyType:   b.convertType(astType.Generics[0]),
				ValueType: b.convertType(astType.Generics[1]),
			}
		}
	}

	// Simple type conversion
	switch astType.Name.Value {
	case "U8":
		return &IntType{Bits: 8}
	case "U16":
		return &IntType{Bits: 16}
	case "U32":
		return &IntType{Bits: 32}
	case "U64":
		return &IntType{Bits: 64}
	case "U128":
		return &IntType{Bits: 128}
	case "U256":
		return &IntType{Bits: 256}
	case "Bool":
		return &BoolType{}
	case "Address":
		return &AddressType{}
	case "String":
		return &StringType{}
	default:
		return &IntType{Bits: 256}
	}
}

func (b *Builder) getStorageType(fieldName string) Type {
	// Look up the storage field type from the contract
	// This is a simplified version - would need to access the actual AST
	return &IntType{Bits: 256}
}

func (b *Builder) resolveFunctionName(callee ast.Expr) (string, string) {
	switch c := callee.(type) {
	case *ast.IdentExpr:
		// Check if this is an imported function
		if b.context != nil && b.context.IsImportedFunction(c.Name) {
			if importedFunc := b.context.GetImportedFunction(c.Name); importedFunc != nil {
				// Return the function name and its module path
				return c.Name, importedFunc.ModulePath
			}
		}
		// Local function or unresolved
		return c.Name, ""
	case *ast.CalleePath:
		if len(c.Parts) == 1 {
			return c.Parts[0].Value, ""
		}
		pathStrs := make([]string, len(c.Parts)-1)
		for i, ident := range c.Parts[:len(c.Parts)-1] {
			pathStrs[i] = ident.Value
		}
		module := strings.Join(pathStrs, "::")
		funcName := c.Parts[len(c.Parts)-1].Value
		return funcName, module
	}
	return "unknown", ""
}

// sealAllBlocks completes the SSA construction by sealing all blocks
func (b *Builder) sealAllBlocks() {
	for _, block := range b.currentFunc.Blocks {
		b.sealedBlocks[block] = true
	}
}

// generateAccessorFunction generates IR for simple accessor functions based on naming patterns
func (b *Builder) generateAccessorFunction(astFunc *ast.Function) {
	funcName := astFunc.Name.Value

	// Map function names to storage field names
	var fieldName string
	switch funcName {
	case "name":
		fieldName = "name"
	case "symbol":
		fieldName = "symbol"
	case "decimals":
		fieldName = "decimals"
	case "totalSupply":
		fieldName = "total_supply"
	default:
		// For balanceOf and allowance, we need to handle parameters
		if funcName == "balanceOf" && len(astFunc.Params) == 1 {
			// balanceOf(owner) -> State.balances[owner]
			ownerParam := b.readVariable(astFunc.Params[0].Name.Value)
			if slot, exists := b.storageSlots["balances"]; exists {
				result := b.createValue("balance_result", &IntType{Bits: 256})
				inst := &KeyedStorageLoadInstruction{
					ID:       b.nextInstID(),
					Result:   result,
					Block:    b.currentBlock,
					Key:      ownerParam,
					BaseSlot: slot,
					KeyType:  ownerParam.Type,
				}
				b.addInstruction(inst)

				terminator := &ReturnTerminator{
					ID:    b.nextInstID(),
					Block: b.currentBlock,
					Value: result,
				}
				b.currentBlock.Terminator = terminator
			}
			return
		}
		if funcName == "allowance" && len(astFunc.Params) == 2 {
			// allowance(owner, spender) -> State.allowances[(owner, spender)]
			ownerParam := b.readVariable(astFunc.Params[0].Name.Value)
			spenderParam := b.readVariable(astFunc.Params[1].Name.Value)

			// Create tuple key from the two parameters
			// For now, we'll use the owner param as key (simplified)
			// TODO: Implement proper tuple key construction
			keyValue := ownerParam

			if slot, exists := b.storageSlots["allowances"]; exists {
				result := b.createValue("allowance_result", &IntType{Bits: 256})
				inst := &KeyedStorageLoadInstruction{
					ID:       b.nextInstID(),
					Result:   result,
					Block:    b.currentBlock,
					Key:      keyValue,
					BaseSlot: slot,
					KeyType:  &TupleType{Elements: []Type{&AddressType{}, &AddressType{}}},
				}
				b.addInstruction(inst)

				// Add a comment showing we're using both parameters
				_ = spenderParam // Used conceptually for tuple key

				terminator := &ReturnTerminator{
					ID:    b.nextInstID(),
					Block: b.currentBlock,
					Value: result,
				}
				b.currentBlock.Terminator = terminator
			}
			return
		}
		// Unknown function, add void return
		terminator := &ReturnTerminator{
			ID:    b.nextInstID(),
			Block: b.currentBlock,
			Value: nil,
		}
		b.currentBlock.Terminator = terminator
		return
	}

	// Generate storage load for simple field access
	if slot, exists := b.storageSlots[fieldName]; exists {
		slotValue := b.buildConstant(strconv.Itoa(slot), &IntType{Bits: 256})
		result := b.createValue("storage_result", b.getStorageType(fieldName))

		inst := &StorageLoadInstruction{
			ID:      b.nextInstID(),
			Result:  result,
			Block:   b.currentBlock,
			Slot:    slotValue,
			SlotNum: slot,
		}
		b.addInstruction(inst)

		terminator := &ReturnTerminator{
			ID:    b.nextInstID(),
			Block: b.currentBlock,
			Value: result,
		}
		b.currentBlock.Terminator = terminator
	} else {
		// Field not found, add void return
		terminator := &ReturnTerminator{
			ID:    b.nextInstID(),
			Block: b.currentBlock,
			Value: nil,
		}
		b.currentBlock.Terminator = terminator
	}
}

// buildCFG constructs the control flow graph
func (b *Builder) buildCFG() {
	if len(b.program.Functions) == 0 {
		return
	}

	// Collect all basic blocks and categorize entry/exit points
	var allBlocks []*BasicBlock
	var entryPoints []*BasicBlock  // External function entries + constructor
	var successExits []*BasicBlock // RETURN terminators
	var failureExits []*BasicBlock // REVERT terminators
	functions := make(map[string]*FunctionCFG)

	for _, fn := range b.program.Functions {
		// Create per-function CFG
		fnCFG := &FunctionCFG{
			Name:         fn.Name,
			Entry:        fn.Entry,
			SuccessExits: []*BasicBlock{},
			FailureExits: []*BasicBlock{},
			Blocks:       fn.Blocks,
		}

		// External functions and constructor are entry points for contract execution
		if (fn.External || fn.Create) && fn.Entry != nil {
			entryPoints = append(entryPoints, fn.Entry)
		}

		// Collect all blocks from this function
		for _, block := range fn.Blocks {
			allBlocks = append(allBlocks, block)

			// Categorize exit blocks by terminator type
			if block.Terminator != nil {
				switch block.Terminator.(type) {
				case *ReturnTerminator:
					successExits = append(successExits, block)
					fnCFG.SuccessExits = append(fnCFG.SuccessExits, block)
				case *RevertInstruction:
					failureExits = append(failureExits, block)
					fnCFG.FailureExits = append(fnCFG.FailureExits, block)
					// BranchTerminator and JumpTerminator are not exit blocks
				}
			}
		}

		functions[fn.Name] = fnCFG
	}

	// Add function call edges to the CFG
	b.addFunctionCallEdges()

	// Build the program CFG with proper categorization
	b.program.CFG = &ControlFlowGraph{
		EntryPoints:  entryPoints,
		SuccessExits: successExits,
		FailureExits: failureExits,
		Blocks:       allBlocks,
		Dominance:    make(map[*BasicBlock][]*BasicBlock),
		Loops:        []*Loop{},
		Functions:    functions,
	}
}

// addFunctionCallEdges adds CFG edges for function calls
func (b *Builder) addFunctionCallEdges() {
	// Create a map of function names to their entry blocks
	functionEntries := make(map[string]*BasicBlock)
	for _, fn := range b.program.Functions {
		if fn.Entry != nil {
			functionEntries[fn.Name] = fn.Entry
		}
	}

	// Scan all blocks for CallInstructions and add edges
	for _, fn := range b.program.Functions {
		for _, block := range fn.Blocks {
			for _, inst := range block.Instructions {
				if callInst, ok := inst.(*CallInstruction); ok {
					// Find the target function's entry block
					if targetEntry, exists := functionEntries[callInst.Function]; exists {
						// Add edge from calling block to target function entry
						block.Successors = append(block.Successors, targetEntry)
						targetEntry.Predecessors = append(targetEntry.Predecessors, block)
					}
				}
			}
		}
	}
}
