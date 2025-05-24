package grammar

import (
	"fmt"
	"strings"
)

func indent(level int) string {
	return strings.Repeat("    ", level)
}

func (p *Program) String() string {
	var b strings.Builder
	for _, s := range p.SourceElements {
		b.WriteString(s.StringWithIndent(0))
	}
	return b.String()
}

func (s *SourceElement) StringWithIndent(level int) string {
	if s.Comment != nil {
		return s.Comment.String() + "\n"
	}
	if s.Module != nil {
		return s.Module.StringWithIndent(level) + "\n"
	}
	return ""
}

func (c *Comment) String() string {
	return c.Text
}

func (m *Module) StringWithIndent(level int) string {
	var b strings.Builder
	if m.DocBeforeAttr != nil {
		b.WriteString(indent(level) + m.DocBeforeAttr.String() + "\n")
	}
	if m.Attribute != nil {
		b.WriteString(m.Attribute.String() + "\n")
	}
	if m.DocAfterAttr != nil {
		b.WriteString(indent(level) + m.DocAfterAttr.String() + "\n")
	}
	b.WriteString(fmt.Sprintf("%smodule %s {\n", indent(level), m.Name))
	for _, u := range m.Uses {
		b.WriteString(indent(level+1) + u.String() + "\n")
	}
	for _, s := range m.Structs {
		b.WriteString(s.StringWithIndent(level + 1))
	}
	for _, f := range m.Functions {
		b.WriteString(f.StringWithIndent(level + 1))
	}
	b.WriteString("}\n")
	return b.String()
}

func (m *ModuleAttribute) String() string {
	return fmt.Sprintf("#[%s]", m.Name)
}

func (u *Use) String() string {
	var ns []string
	for _, n := range u.Namespaces {
		ns = append(ns, n.Name)
	}
	var imports []string
	for _, i := range u.Imports {
		imports = append(imports, i.Name)
	}
	return fmt.Sprintf("use %s::%s;", strings.Join(ns, "::"), strings.Join(imports, ", "))
}

func (s *Struct) StringWithIndent(level int) string {
	var b strings.Builder
	if s.DocBeforeAttr != nil {
		b.WriteString(indent(level) + s.DocBeforeAttr.String() + "\n")
	}
	if s.Attribute != nil {
		b.WriteString(indent(level) + s.Attribute.String() + "\n")
	}
	if s.DocAfterAttr != nil {
		b.WriteString(indent(level) + s.DocAfterAttr.String() + "\n")
	}
	b.WriteString(fmt.Sprintf("%sstruct %s {\n", indent(level), s.Name))
	for _, f := range s.Fields {
		b.WriteString(indent(level+1) + f.String() + "\n")
	}
	b.WriteString(indent(level) + "}\n")
	return b.String()
}

func (d *DocComment) String() string {
	return d.Text
}

func (a *StructAttribute) String() string {
	return fmt.Sprintf("#[%s]", a.Name)
}

func (f *StructField) String() string {
	return fmt.Sprintf("%s: %s,", f.Name, f.Type.String())
}

func (t *Type) String() string {
	if t.Ref != nil {
		return t.Ref.String()
	}
	if len(t.Generics) == 0 {
		return t.Name
	}
	var gens []string
	for _, g := range t.Generics {
		gens = append(gens, g.String())
	}
	return fmt.Sprintf("%s<%s>", t.Name, strings.Join(gens, ", "))
}

func (r *RefType) String() string {
	if r.Mut {
		return fmt.Sprintf("&mut %s", r.Target.String())
	}
	return fmt.Sprintf("&%s", r.Target.String())
}

func (f *Function) StringWithIndent(level int) string {
	var b strings.Builder
	if f.DocBeforeAttr != nil {
		b.WriteString(indent(level) + f.DocBeforeAttr.String() + "\n")
	}
	if f.Attribute != nil {
		b.WriteString(indent(level) + f.Attribute.String() + "\n")
	}
	if f.DocAfterAttr != nil {
		b.WriteString(indent(level) + f.DocAfterAttr.String() + "\n")
	}
	if f.Public {
		b.WriteString(indent(level) + "public ")
	} else {
		b.WriteString(indent(level))
	}
	b.WriteString(fmt.Sprintf("fun %s(", f.Name))
	for i, p := range f.Params {
		if i > 0 {
			b.WriteString(", ")
		}
		b.WriteString(p.String())
	}
	b.WriteString(")")
	if f.Return != nil {
		b.WriteString(fmt.Sprintf(": %s", f.Return.String()))
	}
	if len(f.Reads) > 0 {
		var reads []string
		for _, r := range f.Reads {
			reads = append(reads, r.String())
		}
		b.WriteString(fmt.Sprintf(" reads %s", strings.Join(reads, ", ")))
	}
	if len(f.Writes) > 0 {
		var writes []string
		for _, w := range f.Writes {
			writes = append(writes, w.String())
		}
		b.WriteString(fmt.Sprintf(" writes %s", strings.Join(writes, ", ")))
	}
	b.WriteString(" " + f.Body.StringWithIndent(level))
	return b.String()
}

func (fa *FunctionAttribute) String() string {
	return fmt.Sprintf("#[%s]", fa.Name)
}

func (fp *FunctionParam) String() string {
	return fmt.Sprintf("%s: %s", fp.Name, fp.Type.String())
}

func (fb *FunctionBlock) StringWithIndent(level int) string {
	var b strings.Builder
	b.WriteString("{\n")
	for _, s := range fb.Statements {
		b.WriteString(s.StringWithIndent(level + 1))
	}
	b.WriteString(indent(level) + "}\n")
	return b.String()
}

func (s *Statement) StringWithIndent(level int) string {
	if s.Comment != nil {
		return indent(level) + s.Comment.String() + "\n"
	}
	if s.AssertStmt != nil {
		return indent(level) + s.AssertStmt.String() + "\n"
	}
	if s.LetStmt != nil {
		return indent(level) + s.LetStmt.String() + "\n"
	}
	if s.ReturnStmt != nil {
		return indent(level) + s.ReturnStmt.String() + "\n"
	}
	if s.ExprStmt != nil {
		return indent(level) + s.ExprStmt.String() + "\n"
	}
	if s.AssignStmt != nil {
		return indent(level) + s.AssignStmt.String() + "\n"
	}
	return ""
}

func (l *LetStmt) String() string {
	return fmt.Sprintf("let %s = %s;", l.Name, l.Expr.String())
}

func (a *AssignStmt) String() string {
	var b strings.Builder
	if a.Dereference {
		b.WriteString("*")
	}
	b.WriteString(a.Target)
	b.WriteString(" = ")
	b.WriteString(a.Value.String())
	b.WriteString(";")
	return b.String()
}

func (e *ExprStmt) String() string {
	return fmt.Sprintf("%s;", e.Expr.String())
}

func (a *AssertStmt) String() string {
	var args []string
	for _, arg := range a.Args {
		args = append(args, arg.String())
	}
	return fmt.Sprintf("assert!(%s);", strings.Join(args, ", "))
}

func (r *ReturnStmt) String() string {
	if r.Expr != nil {
		return fmt.Sprintf("return %s;", r.Expr.String())
	}
	return "return;"
}

func (e *Expr) String() string {
	if e.Binary != nil {
		return e.Binary.String()
	}
	return ""
}

func (b *BinaryExpr) String() string {
	s := b.Left.String()
	for _, op := range b.Ops {
		s += " " + op.String()
	}
	return s
}

func (b *BinOp) String() string {
	return fmt.Sprintf("%s %s", b.Operator, b.Right.String())
}

func (u *UnaryExpr) String() string {
	var b strings.Builder
	if u.Operator != nil {
		b.WriteString(*u.Operator)
	}
	if u.Mut {
		b.WriteString("mut ")
	}
	b.WriteString(u.Value.String())
	return b.String()
}

func (p *PostfixExpr) String() string {
	s := p.Primary.String()
	for _, op := range p.Suffix {
		s += op.String()
	}
	return s
}

func (p *PostfixOp) String() string {
	if p.Call != nil {
		return fmt.Sprintf(".%s%s", p.Name, p.Call.String())
	}
	return "." + p.Name
}

func (c *CallSuffix) String() string {
	var b strings.Builder
	b.WriteString("(")
	for i, arg := range c.Args {
		if i > 0 {
			b.WriteString(", ")
		}
		b.WriteString(arg.String())
	}
	b.WriteString(")")
	return b.String()
}

func (m *MethodCall) String() string {
	var b strings.Builder
	b.WriteString("." + m.Name + "(")
	for i, arg := range m.Args {
		if i > 0 {
			b.WriteString(", ")
		}
		b.WriteString(arg.String())
	}
	b.WriteString(")")
	return b.String()
}

func (p *PrimaryExpr) String() string {
	switch {
	case p.Call != nil:
		return p.Call.String()
	case p.Struct != nil:
		return p.Struct.String()
	case p.Number != nil:
		return *p.Number
	case p.Ident != nil:
		return *p.Ident
	case p.Parens != nil:
		return "(" + p.Parens.String() + ")"
	}
	return ""
}

func (c *CallExpr) String() string {
	s := c.Callee.String()
	if len(c.Generic) > 0 {
		var gens []string
		for _, g := range c.Generic {
			gens = append(gens, g.String())
		}
		s += "<" + strings.Join(gens, ", ") + ">"
	}
	s += "("
	for i, arg := range c.Args {
		if i > 0 {
			s += ", "
		}
		s += arg.String()
	}
	s += ")"
	return s
}

func (c *CalleePath) String() string {
	return strings.Join(c.Parts, "::")
}

func (s *StructLiteralExpr) String() string {
	var b strings.Builder
	b.WriteString(s.Name + " { ")
	for i, f := range s.Fields {
		if i > 0 {
			b.WriteString(", ")
		}
		b.WriteString(f.String())
	}
	b.WriteString(" }")
	return b.String()
}

func (f *StructLiteralField) String() string {
	if f.Value != nil {
		return fmt.Sprintf("%s: %s", f.Name, f.Value.String())
	}
	return f.Name
}

func (f *StructFieldFull) String() string {
	return fmt.Sprintf("%s: %s", f.Name, f.Value.String())
}
