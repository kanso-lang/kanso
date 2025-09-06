package ast

import (
	"fmt"
	"strings"
)

func (c *Contract) String() string {
	var b strings.Builder
	for _, item := range c.ContractItems {
		b.WriteString(item.String())
		b.WriteByte('\n')
	}
	return b.String()
}

func (dc *DocComment) String() string {
	return dc.Text
}

func (c *Comment) String() string {
	return c.Text
}

func (bci *BadContractItem) String() string {
	return fmt.Sprintf("BadContractItem: %s", bci.Bad.Message)
}

func (be *BadExpr) String() string {
	return fmt.Sprintf("BadExpr: %s", be.Bad.Message)
}

func (m *Module) String() string {
	var b strings.Builder

	for _, attr := range m.Attributes {
		b.WriteString(attr.String() + "\n")
	}
	b.WriteString(fmt.Sprintf("module %s {\n", m.Name.Value))
	for _, item := range m.ModuleItems {
		b.WriteString("  " + strings.ReplaceAll(item.String(), "\n", "\n  ") + "\n")
	}
	b.WriteString("}")

	return b.String()
}

func (a *Attribute) String() string {
	return fmt.Sprintf("#[%s]", a.Name)
}

func (bmi *BadModuleItem) String() string {
	return fmt.Sprintf("BadModuleItem: %s", bmi.Bad.Message)
}

func (u *Use) String() string {
	var b strings.Builder

	b.WriteString("use ")
	for i, ns := range u.Namespaces {
		b.WriteString(ns.String())

		if i < len(u.Namespaces)-1 {
			b.WriteString("::")
		}
	}

	for i, imp := range u.Imports {
		if i == 0 {
			b.WriteString("::{")
		}

		b.WriteString(imp.String())

		if i < len(u.Imports)-1 {
			b.WriteString(", ")
		}

		if i == len(u.Imports)-1 {
			b.WriteString("}")
		}
	}

	return b.String() + ";"
}

func (ns *Namespace) String() string {
	return ns.Name.Value
}

func (ii *ImportItem) String() string {
	return ii.Name.Value
}

func (s *Struct) String() string {
	var b strings.Builder
	b.WriteString(fmt.Sprintf("struct %s {", s.Name.Value))
	for i, field := range s.Items {
		if i > 0 {
			b.WriteString(", ")
		}
		b.WriteString(field.String())
	}
	b.WriteString("}")
	return b.String()
}

func (sf *StructField) String() string {
	return fmt.Sprintf("%s: %s", sf.Name.Value, sf.VariableType.String())
}

func (f *Function) String() string {
	var b strings.Builder

	if f.Attribute != nil {
		b.WriteString(f.Attribute.String())
		b.WriteString("\n")
	}

	if f.Public {
		b.WriteString("public ")
	}

	b.WriteString("fun ")
	b.WriteString(f.Name.Value)
	b.WriteString("(")
	for i, param := range f.Params {
		if i > 0 {
			b.WriteString(", ")
		}
		b.WriteString(param.String())
	}
	b.WriteString(")")

	if f.Return != nil {
		b.WriteString(": ")
		b.WriteString(f.Return.String())
	}

	if len(f.Reads) > 0 {
		b.WriteString(" reads(")
		for i, id := range f.Reads {
			if i > 0 {
				b.WriteString(", ")
			}
			b.WriteString(id.Value)
		}
		b.WriteString(")")
	}

	if len(f.Writes) > 0 {
		b.WriteString(" writes(")
		for i, id := range f.Writes {
			if i > 0 {
				b.WriteString(", ")
			}
			b.WriteString(id.Value)
		}
		b.WriteString(")")
	}

	b.WriteString(" {\n")

	b.WriteString(f.Body.String())

	b.WriteString(" }\n")
	return b.String()
}

func (fp *FunctionParam) String() string {
	return fmt.Sprintf("%s: %s", fp.Name.Value, fp.Type.String())
}

func (vt *VariableType) String() string {
	var b strings.Builder
	if vt.Ref != nil {
		b.WriteString(vt.Ref.String())
	} else {
		b.WriteString(vt.Name.Value)
		if len(vt.Generics) > 0 {
			b.WriteString("<")
			for i, g := range vt.Generics {
				if i > 0 {
					b.WriteString(", ")
				}
				b.WriteString(g.String())
			}
			b.WriteString(">")
		}
	}
	return b.String()
}

func (rvt *RefVariableType) String() string {
	var b strings.Builder
	b.WriteString("&")
	if rvt.Mut {
		b.WriteString("mut ")
	}
	b.WriteString(rvt.Target.String())
	return b.String()
}

func (b *FunctionBlock) String() string {
	return b.StringIndented("  ")
}

func (b *FunctionBlock) StringIndented(indent string) string {
	var out strings.Builder
	for _, item := range b.Items {
		out.WriteString(indent)
		out.WriteString(item.String())
		out.WriteByte('\n')
	}
	if b.TailExpr != nil {
		out.WriteString(indent)
		out.WriteString(b.TailExpr.String())
		out.WriteByte('\n')
	}
	return out.String()
}

func (e *ExprStmt) String() string {
	s := e.Expr.String()
	if e.Semicolon {
		return s + ";"
	}
	return s
}

func (r *ReturnStmt) String() string {
	if r.Value == nil {
		return "return;"
	}
	return fmt.Sprintf("return %s;", r.Value.String())
}

func (l *LetStmt) String() string {
	return fmt.Sprintf("let %s = %s;", l.Name.Value, l.Expr.String())
}

func (a *AssignStmt) String() string {
	var op string
	switch a.Operator {
	case ASSIGN:
		op = "="
	case PLUS_ASSIGN:
		op = "+="
	case MINUS_ASSIGN:
		op = "-="
	case STAR_ASSIGN:
		op = "*="
	case SLASH_ASSIGN:
		op = "/="
	default:
		op = a.Operator.String()
	}
	return fmt.Sprintf("%s %s %s;", a.Target.String(), op, a.Value.String())
}

func (a *AssertStmt) String() string {
	args := make([]string, len(a.Args))
	for i, arg := range a.Args {
		args[i] = arg.String()
	}
	return fmt.Sprintf("assert!(%s);", strings.Join(args, ", "))
}

func (b *BinaryExpr) String() string {
	return fmt.Sprintf("(%s %s %s)", b.Left.String(), b.Op, b.Right.String())
}

func (u *UnaryExpr) String() string {
	if u.Op == "&" && u.Mut {
		return fmt.Sprintf("(&mut %s)", u.Value.String())
	}
	return fmt.Sprintf("(%s%s)", u.Op, u.Value.String())
}

func (c *CallExpr) String() string {
	var b strings.Builder

	b.WriteString(c.Callee.String()) // now a full Expr

	if len(c.Generic) > 0 {
		b.WriteByte('<')
		for i, g := range c.Generic {
			if i > 0 {
				b.WriteString(", ")
			}
			b.WriteString(g.String())
		}
		b.WriteByte('>')
	}

	b.WriteByte('(')
	for i, a := range c.Args {
		if i > 0 {
			b.WriteString(", ")
		}
		b.WriteString(a.String())
	}
	b.WriteByte(')')
	return b.String()
}

func (f *FieldAccessExpr) String() string {
	return fmt.Sprintf("%s.%s", f.Target.String(), f.Field)
}

func (s *StructLiteralExpr) String() string {
	var b strings.Builder
	b.WriteString(s.Type.String())
	b.WriteString(" {")
	for i, f := range s.Fields {
		if i > 0 {
			b.WriteString(", ")
		}
		b.WriteString(f.String())
	}
	b.WriteString("}")
	return b.String()
}

func (l *LiteralExpr) String() string {
	return l.Value
}

func (i *IdentExpr) String() string {
	return i.Name
}

func (c *CalleePath) String() string {
	parts := make([]string, len(c.Parts))
	for i, p := range c.Parts {
		parts[i] = p.Value
	}
	return strings.Join(parts, "::")
}

func (f *StructLiteralField) String() string {
	return fmt.Sprintf("%s: %s", f.Name.Value, f.Value.String())
}

func (p *ParenExpr) String() string {
	return fmt.Sprintf("(%s)", p.Value.String())
}

func (i *Ident) String() string {
	return i.Value
}
