use core::fmt::{Debug, Formatter, Result as FmtResult};

/// Used elsewhere to identify elements in `Program` values,
/// e.g., in error messages.
pub type StatementIdx = usize;

/// Each identifies:
/// 1. data types,
/// 2. constructors of values in #1, and
/// 3. relations whose members are in #1.
#[derive(Ord, PartialOrd, Clone, PartialEq, Hash, Eq)]
pub struct DomainId(pub String);

/// Each identifies a variable. Used in the context of a rule.
#[derive(Clone, PartialEq, Hash, Eq)]
pub struct VariableId(pub String);

#[derive(Ord, PartialOrd, Debug, Clone, Hash, PartialEq, Eq)]
pub enum Constant {
    Int(i64),
    Str(String),
}

/// "Abstract values" as they may contain variables.
/// See `Atom` (defined in `dynamics.rs`) for the concretized version.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum RuleAtom {
    Variable(VariableId),
    Constant(Constant),
    Construct { did: DomainId, args: Vec<RuleAtom> },
}

#[derive(Debug, Default)]
pub struct Program {
    pub statements: Vec<Statement>,
}

pub enum Statement {
    Decl(DomainId),
    Defn { did: DomainId, params: Vec<DomainId> },
    Rule(Rule),
    Seal(DomainId),
    Emit(DomainId),
}

#[derive(Debug)]
pub struct Rule {
    pub consequents: Vec<RuleAtom>,
    pub antecedents: Vec<RuleLiteral>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Sign {
    Pos,
    Neg,
}
#[derive(Debug)]
pub struct RuleLiteral {
    pub sign: Sign,
    pub ra: RuleAtom,
}

/////////////

pub struct CommaSep<'a, T: Debug + 'a, I: IntoIterator<Item = &'a T> + Clone> {
    pub iter: I,
    pub spaced: bool,
}

impl<'a, T: Debug + 'a, I: IntoIterator<Item = &'a T> + Clone> Debug for CommaSep<'a, T, I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for (i, x) in self.iter.clone().into_iter().enumerate() {
            if i > 0 {
                write!(f, "{}", if self.spaced { ", " } else { "," })?;
            }
            write!(f, "{:?}", x)?;
        }
        Ok(())
    }
}
impl Debug for Statement {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Statement::Rule(Rule { consequents, antecedents }) => {
                write!(f, "rule {:?}", CommaSep { iter: consequents, spaced: true })?;
                if !antecedents.is_empty() {
                    write!(f, " :- {:?}", CommaSep { iter: antecedents, spaced: true })?;
                }
                Ok(())
            }
            Statement::Decl(did) => write!(f, "decl {:?}", did),
            Statement::Emit(did) => write!(f, "emit {:?}", did),
            Statement::Seal(did) => write!(f, "seal {:?}", did),
            Statement::Defn { did, params } => {
                write!(f, "defn {:?}({:?})", did, CommaSep { iter: params, spaced: false })
            }
        }?;
        write!(f, ". ")
    }
}
impl Debug for DomainId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", &self.0)
    }
}

impl Debug for VariableId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", &self.0)
    }
}
