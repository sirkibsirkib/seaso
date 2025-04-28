/// Methods for preprocessing programs at source level (removing line comments) and abstract syntax level (Allocating fresh names for each VariableId("_")).
pub mod preprocessing;

/// Parsers for types in `ast`, e.g., `Program`.
pub mod parse;

/// Statics of Seaso, implementing the checking of well-formedness of programs, and assignment of types to variables.
pub mod statics;

/// Dynamics of Seaso, implementing methods and defining types needed to compute the denotation of a checked program.`
pub mod dynamics;

// pub mod search;

pub mod util;

use crate::lang::util::VecSet;
use std::collections::{HashMap, HashSet};

/////////////////////////////////////////////

#[derive(Eq, PartialEq, Hash, Clone, Debug, Default)]
pub struct Program {
    pub anon_mod_statements: Vec<Statement>,
    pub parts: VecSet<Part>,
}

#[derive(Hash, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Part {
    pub name: PartName,
    pub uses: VecSet<PartName>,
    pub statements: VecSet<Statement>,
}

/// Used (internally) to remember where and how constructors are defined.
pub type DomainDefinitions = HashMap<DomainId, Vec<DomainId>>;

/// These annotate statements, prescribing exactly one type to each variable.
pub type VariableTypes = HashMap<VariableId, DomainId>;

/// A domain identifier, acting as...
/// 1. data types,
/// 2. constructors of values in #1, and
/// 3. relations whose members are in #1.
#[derive(Ord, PartialOrd, Clone, PartialEq, Hash, Eq)]
pub struct DomainId(pub String);

/// Each identifies a variable. Used in the context of a rule.
#[derive(Clone, PartialEq, Hash, Eq, Ord, PartialOrd)]
pub struct VariableId(pub String);

#[derive(Ord, PartialOrd, Clone, Hash, PartialEq, Eq)]
pub enum Constant {
    Int(i64),
    Str(String),
}

/// "Abstract values" as they may contain variables.
/// See `Atom` (defined in `dynamics.rs`) for the concretized version.
#[derive(Clone, PartialEq, Hash, Eq, Ord, PartialOrd)]
pub enum RuleAtom {
    Variable { vid: VariableId, ascription: Option<DomainId> },
    Constant(Constant),
    Construct { did: DomainId, args: Vec<RuleAtom> },
}

// /// A sequence of statements.
// #[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
// pub struct Statements(pub Vec<Statement>);

/// One of five kinds of statement.
#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Statement {
    Decl(Vec<DomainId>),
    Defn { did: DomainId, params: Vec<DomainId> },
    Rule(Rule),
    Seal(DomainId),
    Emit(DomainId),
}

/// A logical implication rule with N conjunctive consequents and N conjunctive antecedents.
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Rule {
    pub consequents: Vec<RuleAtom>,
    pub antecedents: Vec<RuleLiteral>,
}

/// Positive or negative sign, used to negate atoms, forming literals. Newtype for clarity.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Sign {
    Pos,
    Neg,
}

/// A signed atom. These occur as antecedents of rules.
#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct RuleLiteral {
    pub sign: Sign,
    pub ra: RuleAtom,
}

#[derive(Debug)]
pub struct AnnotatedRule {
    pub v2d: VariableTypes,
    pub rule: Rule,
}

#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct PartName(pub String);

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum StatementAt {
    InPart { part_name: PartName },
    AnonPart { statement_index: usize },
}

#[derive(Debug, Default)]
pub struct DomainSealersModifiers {
    pub sealers: HashSet<StatementAt>,
    pub modifiers: HashSet<StatementAt>,
}

#[derive(Debug)]
pub struct EqClasses {
    pub representatives: HashMap<DomainId, DomainId>,
    pub representative_members: HashMap<DomainId, Vec<DomainId>>,
}

#[derive(Debug, Default)]
pub struct ExecutableConfig {
    pub(crate) subconsequence: bool,
}

#[derive(Debug)]
pub struct ExecutableProgram {
    pub(crate) dd: DomainDefinitions,
    pub(crate) annotated_rules: Vec<AnnotatedRule>,
    pub(crate) emissive: HashSet<DomainId>,
    pub(crate) sealers_modifiers: HashMap<DomainId, DomainSealersModifiers>,
    pub(crate) declared_undefined: HashSet<DomainId>,
    pub(crate) used_undeclared: HashSet<DomainId>,
    pub(crate) executable_config: ExecutableConfig,
}
