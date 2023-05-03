use crate::ast::*;
use std::fmt::{Result as FmtResult, Write};

trait Print {
    fn print(&self, s: &mut String) -> FmtResult;
}
///////////////////

impl Statement {
    pub fn printed(&self) -> String {
        let mut s = String::default();
        self.print(&mut s).expect("should work");
        s
    }
}

fn slice_last_iter<T>(slice: &[T]) -> impl Iterator<Item = (&T, bool)> {
    slice.iter().enumerate().rev().map(|(index, item)| (item, index == 0))
}

impl Print for Statement {
    fn print(&self, s: &mut String) -> FmtResult {
        match self {
            Statement::Rule(Rule { consequents, antecedents }) => {
                write!(s, "rule ")?;
                for (consequent, last) in slice_last_iter(consequents) {
                    consequent.print(s)?;
                    if !last {
                        write!(s, ", ")?
                    }
                }
                if !antecedents.is_empty() {
                    write!(s, " :- ")?;
                    for (antecedent, last) in slice_last_iter(antecedents) {
                        antecedent.print(s)?;
                        if !last {
                            write!(s, " ,")?
                        }
                    }
                }
            }
            Statement::Decl(did) => {
                write!(s, "decl ")?;
                did.print(s)?;
            }
            Statement::Emit(did) => {
                write!(s, "emit ")?;
                did.print(s)?;
            }
            Statement::Seal(did) => {
                write!(s, "seal ")?;
                did.print(s)?;
            }
            Statement::Defn { did, params } => {
                write!(s, "defn ")?;
                did.print(s)?;
                write!(s, "(")?;
                for (param, last) in slice_last_iter(params) {
                    param.print(s)?;
                    if !last {
                        write!(s, ",")?
                    }
                }
                write!(s, ")")?;
            }
        };
        write!(s, ". ")
    }
}
impl Print for DomainId {
    fn print(&self, s: &mut String) -> FmtResult {
        write!(s, "{}", &self.0)
    }
}
impl Print for &VariableId {
    fn print(&self, s: &mut String) -> FmtResult {
        write!(s, "{}", &self.0)
    }
}
impl Print for RuleAtom {
    fn print(&self, s: &mut String) -> FmtResult {
        match self {
            RuleAtom::Constant(c) => c.print(s),
            RuleAtom::Construct { did, args } => {
                did.print(s)?;
                write!(s, "(")?;
                for (arg, last) in slice_last_iter(args) {
                    arg.print(s)?;
                    if !last {
                        write!(s, ",")?
                    }
                }
                write!(s, ")")
            }
            RuleAtom::Variable(vid) => vid.print(s),
        }
    }
}
impl Print for Constant {
    fn print(&self, s: &mut String) -> FmtResult {
        match self {
            Constant::Int(c) => write!(s, "{}", c),
            Constant::Str(c) => write!(s, "{:?}", c),
        }
    }
}

impl Print for &RuleLiteral {
    fn print(&self, s: &mut String) -> FmtResult {
        if self.sign == Sign::Neg {
            write!(s, "!")?;
        }
        self.ra.print(s)
    }
}
