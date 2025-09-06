package ast

type Node interface {
	NodePos() Position
	NodeEndPos() Position
	NodeType() NodeType
	String() string

	// Metadata support for debugging and compilation tracking
	GetMetadata() *Metadata
	SetMetadata(*Metadata)
}

func (bci *BadContractItem) NodePos() Position    { return bci.Bad.Pos }
func (bci *BadContractItem) NodeEndPos() Position { return bci.Bad.EndPos }
func (*BadContractItem) NodeType() NodeType       { return BAD_CONTRACT_ITEM }

func (be *BadExpr) NodePos() Position    { return be.Bad.Pos }
func (be *BadExpr) NodeEndPos() Position { return be.Bad.EndPos }
func (*BadExpr) NodeType() NodeType      { return BAD_EXPR }

func (i *Ident) NodePos() Position    { return i.Pos }
func (i *Ident) NodeEndPos() Position { return i.EndPos }
func (*Ident) NodeType() NodeType     { return IDENT }

func (dc *DocComment) NodePos() Position    { return dc.Pos }
func (dc *DocComment) NodeEndPos() Position { return dc.EndPos }
func (*DocComment) NodeType() NodeType      { return DOC_COMMENT }

func (c *Comment) NodePos() Position    { return c.Pos }
func (c *Comment) NodeEndPos() Position { return c.EndPos }
func (*Comment) NodeType() NodeType     { return COMMENT }

func (bmi *BadModuleItem) NodePos() Position    { return bmi.Bad.Pos }
func (bmi *BadModuleItem) NodeEndPos() Position { return bmi.Bad.EndPos }
func (*BadModuleItem) NodeType() NodeType       { return BAD_MODULE_ITEM }

func (m *Module) NodePos() Position    { return m.Pos }
func (m *Module) NodeEndPos() Position { return m.EndPos }
func (*Module) NodeType() NodeType     { return MODULE }

func (a *Attribute) NodePos() Position    { return a.Pos }
func (a *Attribute) NodeEndPos() Position { return a.EndPos }
func (*Attribute) NodeType() NodeType     { return ATTRIBUTE }

func (u *Use) NodePos() Position    { return u.Pos }
func (u *Use) NodeEndPos() Position { return u.EndPos }
func (*Use) NodeType() NodeType     { return USE }

func (ns *Namespace) NodePos() Position    { return ns.Pos }
func (ns *Namespace) NodeEndPos() Position { return ns.EndPos }
func (*Namespace) NodeType() NodeType      { return NAMESPACE }

func (ii *ImportItem) NodePos() Position    { return ii.Pos }
func (ii *ImportItem) NodeEndPos() Position { return ii.EndPos }
func (*ImportItem) NodeType() NodeType      { return IMPORT_ITEM }

func (s *Struct) NodePos() Position    { return s.Pos }
func (s *Struct) NodeEndPos() Position { return s.EndPos }
func (*Struct) NodeType() NodeType     { return STRUCT }

func (sf *StructField) NodePos() Position    { return sf.Pos }
func (sf *StructField) NodeEndPos() Position { return sf.EndPos }
func (*StructField) NodeType() NodeType      { return STRUCT_FIELD }

func (t *VariableType) NodePos() Position    { return t.Pos }
func (t *VariableType) NodeEndPos() Position { return t.EndPos }
func (*VariableType) NodeType() NodeType     { return TYPE }

func (rt *RefVariableType) NodePos() Position    { return rt.Pos }
func (rt *RefVariableType) NodeEndPos() Position { return rt.EndPos }
func (*RefVariableType) NodeType() NodeType      { return REF_TYPE }

func (f *Function) NodePos() Position    { return f.Pos }
func (f *Function) NodeEndPos() Position { return f.EndPos }
func (*Function) NodeType() NodeType     { return FUNCTION }

func (fp *FunctionParam) NodePos() Position    { return fp.Pos }
func (fp *FunctionParam) NodeEndPos() Position { return fp.EndPos }
func (*FunctionParam) NodeType() NodeType      { return FUNCTION_PARAM }

func (b *FunctionBlock) NodePos() Position    { return b.Pos }
func (b *FunctionBlock) NodeEndPos() Position { return b.EndPos }
func (*FunctionBlock) NodeType() NodeType     { return FUNCTION_BLOCK }

func (e *ExprStmt) NodePos() Position    { return e.Pos }
func (e *ExprStmt) NodeEndPos() Position { return e.EndPos }
func (*ExprStmt) NodeType() NodeType     { return EXPR_STMT }

func (r *ReturnStmt) NodePos() Position    { return r.Pos }
func (r *ReturnStmt) NodeEndPos() Position { return r.EndPos }
func (*ReturnStmt) NodeType() NodeType     { return RETURN_STMT }

func (l *LetStmt) NodePos() Position    { return l.Pos }
func (l *LetStmt) NodeEndPos() Position { return l.EndPos }
func (*LetStmt) NodeType() NodeType     { return LET_STMT }

func (a *AssignStmt) NodePos() Position    { return a.Pos }
func (a *AssignStmt) NodeEndPos() Position { return a.EndPos }
func (*AssignStmt) NodeType() NodeType     { return ASSIGN_STMT }

func (a *AssertStmt) NodePos() Position    { return a.Pos }
func (a *AssertStmt) NodeEndPos() Position { return a.EndPos }
func (*AssertStmt) NodeType() NodeType     { return ASSERT_STMT }

func (b *BinaryExpr) NodePos() Position    { return b.Pos }
func (b *BinaryExpr) NodeEndPos() Position { return b.EndPos }
func (*BinaryExpr) NodeType() NodeType     { return BINARY_EXPR }

func (u *UnaryExpr) NodePos() Position    { return u.Pos }
func (u *UnaryExpr) NodeEndPos() Position { return u.EndPos }
func (*UnaryExpr) NodeType() NodeType     { return UNARY_EXPR }

func (c *CallExpr) NodePos() Position    { return c.Pos }
func (c *CallExpr) NodeEndPos() Position { return c.EndPos }
func (*CallExpr) NodeType() NodeType     { return CALL_EXPR }

func (f *FieldAccessExpr) NodePos() Position    { return f.Pos }
func (f *FieldAccessExpr) NodeEndPos() Position { return f.EndPos }
func (*FieldAccessExpr) NodeType() NodeType     { return FIELD_ACCESS_EXPR }

func (s *StructLiteralExpr) NodePos() Position    { return s.Pos }
func (s *StructLiteralExpr) NodeEndPos() Position { return s.EndPos }
func (*StructLiteralExpr) NodeType() NodeType     { return STRUCT_LITERAL_EXPR }

func (l *LiteralExpr) NodePos() Position    { return l.Pos }
func (l *LiteralExpr) NodeEndPos() Position { return l.EndPos }
func (*LiteralExpr) NodeType() NodeType     { return LITERAL_EXPR }

func (i *IdentExpr) NodePos() Position    { return i.Pos }
func (i *IdentExpr) NodeEndPos() Position { return i.EndPos }
func (*IdentExpr) NodeType() NodeType     { return IDENT_EXPR }

func (c *CalleePath) NodePos() Position    { return c.Pos }
func (c *CalleePath) NodeEndPos() Position { return c.EndPos }
func (*CalleePath) NodeType() NodeType     { return CALLEE_PATH }

func (f *StructLiteralField) NodePos() Position    { return f.Pos }
func (f *StructLiteralField) NodeEndPos() Position { return f.EndPos }
func (*StructLiteralField) NodeType() NodeType     { return STRUCT_LITERAL_FIELD }

func (p *ParenExpr) NodePos() Position    { return p.Pos }
func (p *ParenExpr) NodeEndPos() Position { return p.EndPos }
func (p *ParenExpr) NodeType() NodeType   { return PAREN_EXPR }

// GetMetadata and SetMetadata implementations for all AST nodes

func (bci *BadContractItem) GetMetadata() *Metadata  { return bci.Bad.metadata }
func (bci *BadContractItem) SetMetadata(m *Metadata) { bci.Bad.metadata = m }

func (be *BadExpr) GetMetadata() *Metadata  { return be.Bad.metadata }
func (be *BadExpr) SetMetadata(m *Metadata) { be.Bad.metadata = m }

func (i *Ident) GetMetadata() *Metadata  { return i.metadata }
func (i *Ident) SetMetadata(m *Metadata) { i.metadata = m }

func (dc *DocComment) GetMetadata() *Metadata  { return dc.metadata }
func (dc *DocComment) SetMetadata(m *Metadata) { dc.metadata = m }

func (c *Comment) GetMetadata() *Metadata  { return c.metadata }
func (c *Comment) SetMetadata(m *Metadata) { c.metadata = m }

func (bmi *BadModuleItem) GetMetadata() *Metadata  { return bmi.Bad.metadata }
func (bmi *BadModuleItem) SetMetadata(m *Metadata) { bmi.Bad.metadata = m }

func (m *Module) GetMetadata() *Metadata     { return m.metadata }
func (m *Module) SetMetadata(meta *Metadata) { m.metadata = meta }

func (a *Attribute) GetMetadata() *Metadata  { return a.metadata }
func (a *Attribute) SetMetadata(m *Metadata) { a.metadata = m }

func (u *Use) GetMetadata() *Metadata  { return u.metadata }
func (u *Use) SetMetadata(m *Metadata) { u.metadata = m }

func (ns *Namespace) GetMetadata() *Metadata  { return ns.metadata }
func (ns *Namespace) SetMetadata(m *Metadata) { ns.metadata = m }

func (ii *ImportItem) GetMetadata() *Metadata  { return ii.metadata }
func (ii *ImportItem) SetMetadata(m *Metadata) { ii.metadata = m }

func (s *Struct) GetMetadata() *Metadata  { return s.metadata }
func (s *Struct) SetMetadata(m *Metadata) { s.metadata = m }

func (sf *StructField) GetMetadata() *Metadata  { return sf.metadata }
func (sf *StructField) SetMetadata(m *Metadata) { sf.metadata = m }

func (t *VariableType) GetMetadata() *Metadata  { return t.metadata }
func (t *VariableType) SetMetadata(m *Metadata) { t.metadata = m }

func (rt *RefVariableType) GetMetadata() *Metadata  { return rt.metadata }
func (rt *RefVariableType) SetMetadata(m *Metadata) { rt.metadata = m }

func (f *Function) GetMetadata() *Metadata  { return f.metadata }
func (f *Function) SetMetadata(m *Metadata) { f.metadata = m }

func (fp *FunctionParam) GetMetadata() *Metadata  { return fp.metadata }
func (fp *FunctionParam) SetMetadata(m *Metadata) { fp.metadata = m }

func (b *FunctionBlock) GetMetadata() *Metadata  { return b.metadata }
func (b *FunctionBlock) SetMetadata(m *Metadata) { b.metadata = m }

func (e *ExprStmt) GetMetadata() *Metadata  { return e.metadata }
func (e *ExprStmt) SetMetadata(m *Metadata) { e.metadata = m }

func (r *ReturnStmt) GetMetadata() *Metadata  { return r.metadata }
func (r *ReturnStmt) SetMetadata(m *Metadata) { r.metadata = m }

func (l *LetStmt) GetMetadata() *Metadata  { return l.metadata }
func (l *LetStmt) SetMetadata(m *Metadata) { l.metadata = m }

func (a *AssignStmt) GetMetadata() *Metadata  { return a.metadata }
func (a *AssignStmt) SetMetadata(m *Metadata) { a.metadata = m }

func (a *AssertStmt) GetMetadata() *Metadata  { return a.metadata }
func (a *AssertStmt) SetMetadata(m *Metadata) { a.metadata = m }

func (b *BinaryExpr) GetMetadata() *Metadata  { return b.metadata }
func (b *BinaryExpr) SetMetadata(m *Metadata) { b.metadata = m }

func (u *UnaryExpr) GetMetadata() *Metadata  { return u.metadata }
func (u *UnaryExpr) SetMetadata(m *Metadata) { u.metadata = m }

func (c *CallExpr) GetMetadata() *Metadata  { return c.metadata }
func (c *CallExpr) SetMetadata(m *Metadata) { c.metadata = m }

func (f *FieldAccessExpr) GetMetadata() *Metadata  { return f.metadata }
func (f *FieldAccessExpr) SetMetadata(m *Metadata) { f.metadata = m }

func (s *StructLiteralExpr) GetMetadata() *Metadata  { return s.metadata }
func (s *StructLiteralExpr) SetMetadata(m *Metadata) { s.metadata = m }

func (l *LiteralExpr) GetMetadata() *Metadata  { return l.metadata }
func (l *LiteralExpr) SetMetadata(m *Metadata) { l.metadata = m }

func (i *IdentExpr) GetMetadata() *Metadata  { return i.metadata }
func (i *IdentExpr) SetMetadata(m *Metadata) { i.metadata = m }

func (c *CalleePath) GetMetadata() *Metadata  { return c.metadata }
func (c *CalleePath) SetMetadata(m *Metadata) { c.metadata = m }

func (f *StructLiteralField) GetMetadata() *Metadata  { return f.metadata }
func (f *StructLiteralField) SetMetadata(m *Metadata) { f.metadata = m }

func (p *ParenExpr) GetMetadata() *Metadata  { return p.metadata }
func (p *ParenExpr) SetMetadata(m *Metadata) { p.metadata = m }
