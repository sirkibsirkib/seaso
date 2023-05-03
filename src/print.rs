use crate::ast::*;
use std::collections::HashMap;
use std::fmt::{Result as FmtResult, Write};

type Key = u32;

struct Allocator<T> {
    item_to_key: HashMap<T, u32>,
    next: Key,
}

#[derive(Default)]
pub struct DenseRenamer {
    c: Allocator<Constant>,
    d: Allocator<DomainId>,
    v: Allocator<(StatementIdx, VariableId)>,
}

pub enum Mode {
    Dense(DenseRenamer),
    Readable,
}

trait Print {
    fn print(&self, m: &mut Mode, s: &mut String) -> FmtResult;
}
///////////////////
impl<T> Default for Allocator<T> {
    fn default() -> Self {
        Self { item_to_key: Default::default(), next: 0 }
    }
}
impl Program {
    pub fn printed(&self, readable: bool) -> String {
        let mut m = if readable { Mode::Readable } else { Mode::Dense(DenseRenamer::default()) };
        let mut s = String::default();
        self.print(&mut m, &mut s).expect("should work");
        s
    }
}
impl Mode {
    fn dense(&self) -> bool {
        match self {
            Mode::Readable => false,
            Mode::Dense(..) => true,
        }
    }
    fn separator(&self) -> &'static str {
        if self.dense() {
            ",	"
        } else {
            ", "
        }
    }
    fn ender(&self) -> &'static str {
        if self.dense() {
            ""
        } else {
            ""
        }
    }
    fn parens(&self) -> [&'static str; 2] {
        if self.dense() {
            ["(", ")"]
        } else {
            ["(", ")"]
        }
    }
}
impl<T: Clone + Eq + PartialEq + core::hash::Hash> Allocator<T> {
    fn get(&mut self, item: T) -> Key {
        *self.item_to_key.entry(item).or_insert_with(|| {
            self.next += 1;
            self.next - 1
        })
    }
}

impl Print for Program {
    fn print(&self, m: &mut Mode, s: &mut String) -> FmtResult {
        for pair in self.statements.iter().enumerate() {
            pair.print(m, s)?;
        }
        Ok(())
    }
}

fn slice_last_iter<T>(slice: &[T]) -> impl Iterator<Item = (&T, bool)> {
    slice.iter().enumerate().rev().map(|(index, item)| (item, index == 0))
}

impl Print for (StatementIdx, &Statement) {
    fn print(&self, m: &mut Mode, s: &mut String) -> FmtResult {
        let prefix = match self.1 {
            Statement::Rule { .. } => {
                if m.dense() {
                    ":"
                } else {
                    "rule "
                }
            }
            Statement::Defn { .. } => {
                if m.dense() {
                    ":"
                } else {
                    "defn "
                }
            }
            Statement::Decl(..) => {
                if m.dense() {
                    "*"
                } else {
                    "decl "
                }
            }
            Statement::Emit(..) => {
                if m.dense() {
                    "?"
                } else {
                    "emit "
                }
            }
            Statement::Seal(..) => {
                if m.dense() {
                    "!"
                } else {
                    "seal "
                }
            }
        };
        write!(s, "{}", prefix)?;
        match &self.1 {
            Statement::Rule(Rule { consequents, antecedents }) => {
                for (consequent, last) in slice_last_iter(consequents) {
                    (self.0, consequent).print(m, s)?;
                    write!(s, "{}", if last { m.ender() } else { m.separator() })?;
                }
                if !antecedents.is_empty() {
                    let arrow = match &m {
                        Mode::Readable => " :- ",
                        Mode::Dense(..) => "<",
                    };
                    write!(s, "{}", arrow)?;
                    for (antecedent, last) in slice_last_iter(antecedents) {
                        (self.0, antecedent).print(m, s)?;
                        write!(s, "{}", if last { m.ender() } else { m.separator() })?;
                    }
                }
            }
            Statement::Decl(did) | Statement::Emit(did) | Statement::Seal(did) => {
                did.print(m, s)?;
            }
            Statement::Defn { did, params } => {
                did.print(m, s)?;
                write!(s, "{}", m.parens()[0])?;
                for (param, last) in slice_last_iter(params) {
                    param.print(m, s)?;
                    write!(s, "{}", if last { m.ender() } else { m.separator() })?;
                }
                write!(s, "{}", m.parens()[1])?;
            }
        };
        write!(s, "{}", if m.dense() { "" } else { ". " })
    }
}
impl Print for DomainId {
    fn print(&self, m: &mut Mode, s: &mut String) -> FmtResult {
        match m {
            Mode::Readable => write!(s, "{}", &self.0),
            Mode::Dense(r) => write!(s, "{}", r.d.get(self.clone())),
        }
    }
}
impl Print for (StatementIdx, &VariableId) {
    fn print(&self, m: &mut Mode, s: &mut String) -> FmtResult {
        match m {
            Mode::Readable => write!(s, "{}", &(self.1).0),
            Mode::Dense(r) => write!(s, "{}", r.v.get((self.0, self.1.clone()))),
        }
    }
}
impl Print for (StatementIdx, &RuleAtom) {
    fn print(&self, m: &mut Mode, s: &mut String) -> FmtResult {
        match self.1 {
            RuleAtom::Constant(c) => c.print(m, s),
            RuleAtom::Construct { did, args } => {
                did.print(m, s)?;
                write!(s, "{}", m.parens()[0])?;
                for (arg, last) in slice_last_iter(args) {
                    (self.0, arg).print(m, s)?;
                    write!(s, "{}", if last { m.ender() } else { m.separator() })?;
                }
                write!(s, "{}", m.parens()[1])?;
                Ok(())
            }
            RuleAtom::Variable(vid) => (self.0, vid).print(m, s),
        }
    }
}
impl Print for Constant {
    fn print(&self, m: &mut Mode, s: &mut String) -> FmtResult {
        match m {
            Mode::Readable => match self {
                Constant::Int(c) => write!(s, "{}", c),
                Constant::Str(c) => write!(s, "{:?}", c),
            },
            Mode::Dense(r) => {
                write!(s, "{}", r.c.get(self.clone()))
            }
        }
    }
}

impl Print for (StatementIdx, &RuleLiteral) {
    fn print(&self, m: &mut Mode, s: &mut String) -> FmtResult {
        if self.1.sign == Sign::Neg {
            write!(s, "!")?;
        }
        (self.0, &self.1.ra).print(m, s)
    }
}
