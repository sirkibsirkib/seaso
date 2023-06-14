use crate::*;
use core::fmt::{Debug, Formatter, Result as FmtResult};

/// Newtype that suppresses pretty-printing of the wrapped type.
/// Useful in avoiding excessive indentation of internals when pretty printing its container.
pub(crate) struct NoPretty<T: Debug>(pub T);

/// Structure used in debug printing. Prints elements separated by commas.
pub(crate) struct CommaSep<'a, T: Debug + 'a, I: IntoIterator<Item = &'a T> + Clone> {
    pub iter: I,
    pub spaced: bool,
}

impl<T: Debug> Debug for NoPretty<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", &self.0)
    }
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

impl Debug for RuleAtom {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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
