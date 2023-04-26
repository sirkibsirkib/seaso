pub type StatementIdx = usize;

#[derive(Clone, PartialEq, Hash, Eq)]
pub struct DomainId(pub String);

#[derive(Clone, PartialEq, Hash, Eq)]
pub struct VariableId(String);

#[derive(Clone, PartialEq, Hash, Eq)]
pub enum RuleAtom {
    Variable { vid: VariableId },
    IntConst { c: i64 },
    StrConst { c: String },
    Construct { did: DomainId, args: Vec<RuleAtom> },
}

pub struct Program {
    pub statements: Vec<Statement>,
}
pub enum Statement {
    Decl { did: DomainId },
    Defn { did: DomainId, params: Vec<DomainId> },
    Rule { consequents: Vec<RuleAtom>, antecedents: Vec<RuleLiteral> },
    Seal { did: DomainId },
    Emit { did: DomainId },
}

#[derive(Clone)]
pub enum Sign {
    Pos,
    Neg,
}
pub struct RuleLiteral {
    pub sign: Sign,
    pub ra: RuleAtom,
}
