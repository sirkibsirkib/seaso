use core::fmt::{Debug, Formatter, Result as FmtResult};

/// Used elsewhere to identify elements in `Program` values,
/// e.g., in error messages.
pub type StatementIdx = usize;

/// A domain identifier, acting as...
/// 1. data types,
/// 2. constructors of values in #1, and
/// 3. relations whose members are in #1.
#[derive(Ord, PartialOrd, Clone, PartialEq, Hash, Eq)]
pub struct DomainId(pub String);

/// Each identifies a variable. Used in the context of a rule.
#[derive(Clone, PartialEq, Hash, Eq)]
pub struct VariableId(pub String);

#[derive(Ord, PartialOrd, Clone, Hash, PartialEq, Eq)]
pub enum Constant {
    Int(i64),
    Str(String),
}

/// "Abstract values" as they may contain variables.
/// See `Atom` (defined in `dynamics.rs`) for the concretized version.
#[derive(Clone, PartialEq, Hash, Eq)]
pub enum RuleAtom {
    Variable(VariableId),
    Constant(Constant),
    Construct { did: DomainId, args: Vec<RuleAtom> },
}

/// A sequence of statements.
#[derive(Debug, Default)]
pub struct Program {
    pub statements: Vec<Statement>,
}

/// One of five kinds of statement.
pub enum Statement {
    Decl(DomainId),
    Defn { did: DomainId, params: Vec<DomainId> },
    Rule(Rule),
    Seal(DomainId),
    Emit(DomainId),
}

/// A logical implication rule with N conjunctive consequents and N conjunctive antecedents.
#[derive(Debug)]
pub struct Rule {
    pub consequents: Vec<RuleAtom>,
    pub antecedents: Vec<RuleLiteral>,
}

/// Positive or negative sign, used to negate atoms, forming literals. Newtype for clarity.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Sign {
    Pos,
    Neg,
}

/// A signed atom. These occur as antecedents of rules.
pub struct RuleLiteral {
    pub sign: Sign,
    pub ra: RuleAtom,
}

/////////////

impl Debug for RuleAtom {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use crate::util::CommaSep;
        match self {
            Self::Variable(vid) => vid.fmt(f),
            Self::Constant(c) => c.fmt(f),
            Self::Construct { did, args } => {
                write!(f, "{:?}({:?})", did, CommaSep { iter: args, spaced: false })
            }
        }
    }
}
impl Debug for RuleLiteral {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.sign == Sign::Neg {
            write!(f, "!")?
        }
        self.ra.fmt(f)
    }
}

impl Debug for Statement {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use crate::util::CommaSep;
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

impl Debug for Constant {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Int(c) => c.fmt(f),
            Self::Str(c) => c.fmt(f),
        }
    }
}
impl Debug for VariableId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", &self.0)
    }
}
