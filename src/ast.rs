pub type StatementIdx = usize;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct DomainId(pub String);

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct VariableId(pub String);

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum RuleAtom {
    Variable { vid: VariableId },
    IntConst { c: i64 },
    StrConst { c: String },
    Construct { did: DomainId, args: Vec<RuleAtom> },
}

#[derive(Debug, Default)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Decl { did: DomainId },
    Defn { did: DomainId, params: Vec<DomainId> },
    Rule(Rule),
    Seal { did: DomainId },
    Emit { did: DomainId },
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
