use crate::ast::*;

use std::collections::{HashMap, HashSet};

/// Used (internally) to remember where and how constructors are defined.
type DomainDefinitions = HashMap<DomainId, (StatementIdx, Vec<DomainId>)>;

/// These annotate programs to create the internal representation.
/// Ultimately, they prescribe exactly one type to each variable in each rule.
pub type RuleVariableTypes = HashMap<StatementIdx, VariableTypes>;

/// These annotate statements, prescribing exactly one type to each variable.
pub type VariableTypes = HashMap<VariableId, DomainId>;

/// Error resulting from checking a program. Bundles the `StatementCheckErr` with a statement and statement index for convenient debugging.
#[derive(Debug)]
pub struct CheckErr<'a> {
    pub statement_index: StatementIdx,
    pub statement: &'a Statement,
    pub error: StatementCheckErr,
}

/// Error within a particular statement resulting from checking a program.
#[derive(Debug)]
pub enum StatementCheckErr {
    UndefinedConstructor(DomainId),
    OneVariableTwoTypes { vid: VariableId, domains: [DomainId; 2] },
    NoTypes(VariableId),
    WrongArity { did: DomainId, param_count: usize, arg_count: usize },
    VariableNotEnumerable(VariableId),
    ConflictingDefinition { did: DomainId, previous_sidx: StatementIdx },
    MistypedArgument { constructor: DomainId, expected: DomainId, got: DomainId },
    DefiningPrimitive(DomainId),
}

/// Identifies which statements first seal and then modify which domain.
#[derive(Debug)]
pub struct SealBreak {
    pub sealed: StatementIdx,
    pub modified: StatementIdx,
    pub did: DomainId,
}

/// Bundles together a `Program` and a `RuleVariableTypes` produced by checking it.
#[derive(Debug)]
pub struct Checked<'a> {
    pub(crate) program: &'a Program,
    pub(crate) rvt: RuleVariableTypes,
}

impl DomainId {
    pub const PRIMITIVE_STRS: [&str; 2] = ["str", "int"];
}

impl Into<RuleVariableTypes> for Checked<'_> {
    fn into(self) -> RuleVariableTypes {
        self.rvt
    }
}
impl AsRef<RuleVariableTypes> for Checked<'_> {
    fn as_ref(&self) -> &RuleVariableTypes {
        &self.rvt
    }
}

impl Program {
    /// Used to filter the truths of the denotation.
    pub fn emitted_domains(&self) -> HashSet<&DomainId> {
        self.statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Emit(did) => Some(did),
                _ => None,
            })
            .collect()
    }

    /// Check all well-formedness criteria of this program depended on
    /// by `denotation`. Returns the assignment of a unique `DomainId`
    /// per variable per rule. `(Program,RuleVariableTypes)` can be
    /// understood as the internal representation.
    /// Note, the other well-formedness criteria can be checked separately,
    /// e.g., using `undeclared_domains`.
    pub fn check(&self) -> Result<Checked, CheckErr> {
        let dd = self.domain_definitions()?;
        self.statements
            .iter()
            .enumerate()
            .flat_map(|(sidx, statement)| {
                statement.rule_type_variables(&dd).map(|x| {
                    x.map(|x| (sidx, x)).map_err(|error| CheckErr {
                        statement_index: sidx,
                        statement: &self.statements[sidx],
                        error,
                    })
                })
            })
            .collect::<Result<_, _>>()
            .map(|rvt| Checked { program: self, rvt })
    }

    /// Returns all domains used but not declared, which is trivially
    /// adapted to enforce the `all used types are declared` well-formedness criterion.
    pub fn undeclared_domains(&self) -> HashSet<&DomainId> {
        // step 2: all occurring constructs are declared
        let declared = self.declarations();
        let mut occurring = HashSet::default();
        for statement in &self.statements {
            statement.occurring_domain_ids(&mut occurring)
        }
        occurring.retain(|did| !declared.contains(did));
        occurring
    }

    /// Checks the `mut-after-seal` well-formedness criterion.
    pub fn seal_break(&self) -> Option<SealBreak> {
        type LastSealedAt = HashMap<DomainId, StatementIdx>;
        let mut lsa = LastSealedAt::default();
        for (sidx, statement) in self.statements.iter().enumerate() {
            match statement {
                Statement::Seal(did) => {
                    lsa.insert(did.clone(), sidx);
                }
                Statement::Rule(Rule { consequents, .. }) => {
                    for consequent in consequents {
                        if let RuleAtom::Construct { did, .. } = consequent {
                            if let Some(&s_sidx) = lsa.get(did) {
                                return Some(SealBreak {
                                    sealed: s_sidx,
                                    modified: sidx,
                                    did: did.clone(),
                                });
                            }
                        }
                    }
                }
                Statement::Emit(did) => {
                    if let Some(&s_sidx) = lsa.get(did) {
                        return Some(SealBreak {
                            sealed: s_sidx,
                            modified: sidx,
                            did: did.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Returns the unique definition of each defined domain
    pub fn domain_definitions(&self) -> Result<DomainDefinitions, CheckErr> {
        let mut dd = DomainDefinitions::default();
        let mut f = |sidx, statement: &Statement| {
            if let Statement::Defn { did, params } = statement {
                if did.is_primitive() {
                    return Err(StatementCheckErr::DefiningPrimitive(did.clone()));
                }
                let value = (sidx, params.clone());
                let prev = dd.insert(did.clone(), value.clone());
                if let Some((previous_sidx, previous_params)) = prev {
                    if &previous_params != params {
                        return Err(StatementCheckErr::ConflictingDefinition {
                            previous_sidx,
                            did: did.clone(),
                        });
                    }
                }
            }
            Ok(())
        };
        for (sidx, statement) in self.statements.iter().enumerate() {
            f(sidx, statement).map_err(|error| CheckErr {
                statement_index: sidx,
                statement: &self.statements[sidx],
                error,
            })?
        }
        Ok(dd)
    }

    /// Returns the set of all declared domains.
    pub fn declarations(&self) -> HashSet<DomainId> {
        self.statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Decl(did) | Statement::Defn { did, .. } => Some(did.clone()),
                _ => None,
            })
            .chain(DomainId::PRIMITIVE_STRS.map(String::from).map(DomainId))
            .collect()
    }
}

impl DomainId {
    pub fn is_primitive(&self) -> bool {
        Self::PRIMITIVE_STRS.as_slice().contains(&self.0.as_ref())
    }
}

impl Statement {
    /// Returns all domain IDs within. Used to check if all are declared.
    fn occurring_domain_ids<'a>(&'a self, x: &mut HashSet<&'a DomainId>) {
        match self {
            Statement::Defn { did, params } => {
                x.insert(did);
                for param in params {
                    x.insert(param);
                }
            }
            Statement::Decl(did) | Statement::Emit(did) | Statement::Seal(did) => {
                x.insert(did);
            }
            Statement::Rule(..) => {}
        }
    }

    /// Returns a unique, consistent assignment to each occurring variable, if they exist.
    fn rule_type_variables(
        &self,
        dd: &DomainDefinitions,
    ) -> Option<Result<VariableTypes, StatementCheckErr>> {
        if let Statement::Rule(Rule { consequents, antecedents }) = self {
            let mut vt = VariableTypes::default();
            let mut vids = HashSet::<VariableId>::default();
            for ra in consequents.iter().chain(antecedents.iter().map(|lit| &lit.ra)) {
                ra.variables(&mut vids);
                if let Err(err) = ra.type_variables(dd, &mut vt) {
                    return Some(Err(err));
                }
            }
            Some(if let Some(vid) = vids.iter().find(|&vid| !vt.contains_key(vid)) {
                Err(StatementCheckErr::NoTypes(vid.clone()))
            } else {
                // enumerability check
                let mut enumerable = HashSet::default();
                for antecedent in antecedents {
                    if let RuleLiteral { sign: Sign::Pos, ra } = antecedent {
                        ra.variables(&mut enumerable);
                    }
                }
                if let Some(vid) = vids.difference(&enumerable).next() {
                    Err(StatementCheckErr::VariableNotEnumerable(vid.clone()))
                } else {
                    Ok(vt)
                }
            })
        } else {
            None
        }
    }
}

impl Constant {
    pub fn domain_id(&self) -> DomainId {
        DomainId(
            match self {
                Self::Int { .. } => "int",
                Self::Str { .. } => "str",
            }
            .to_owned(),
        )
    }
}

impl RuleAtom {
    pub fn apparent_did(&self) -> Option<DomainId> {
        match self {
            RuleAtom::Construct { did, .. } => Some(did.clone()),
            RuleAtom::Constant(c) => Some(c.domain_id()),
            RuleAtom::Variable { .. } => None,
        }
    }
    fn type_variables(
        &self,
        dd: &DomainDefinitions,
        vt: &mut VariableTypes,
    ) -> Result<(), StatementCheckErr> {
        if let RuleAtom::Construct { did, args } = self {
            let (_defn_sidx, param_dids) =
                dd.get(did).ok_or(StatementCheckErr::UndefinedConstructor(did.clone()))?;
            if param_dids.len() != args.len() {
                return Err(StatementCheckErr::WrongArity {
                    did: did.clone(),
                    param_count: param_dids.len(),
                    arg_count: args.len(),
                });
            }
            for (arg, param_did) in args.iter().zip(param_dids.iter()) {
                if let RuleAtom::Variable(vid) = arg {
                    match vt.insert(vid.clone(), param_did.clone()) {
                        Some(param_did2) if param_did != &param_did2 => {
                            return Err(StatementCheckErr::OneVariableTwoTypes {
                                vid: vid.clone(),
                                domains: [param_did.clone(), param_did2.clone()],
                            });
                        }
                        _ => {}
                    }
                } else if let Some(arg_did) = arg.apparent_did() {
                    if &arg_did != param_did {
                        return Err(StatementCheckErr::MistypedArgument {
                            constructor: did.clone(),
                            expected: param_did.clone(),
                            got: arg_did,
                        });
                    }
                }
            }
            for arg in args {
                arg.type_variables(dd, vt)?
            }
        }
        Ok(())
    }
    fn variables(&self, vids: &mut HashSet<VariableId>) {
        match self {
            RuleAtom::Constant { .. } => {}
            RuleAtom::Variable(vid) => drop(vids.insert(vid.clone())),
            RuleAtom::Construct { args, .. } => {
                for arg in args {
                    arg.variables(vids)
                }
            }
        }
    }
    pub fn domain_id(&self, vt: &VariableTypes) -> Result<DomainId, ()> {
        match self {
            RuleAtom::Construct { did, .. } => Ok(did.clone()),
            RuleAtom::Variable(vid) => vt.get(vid).ok_or(()).cloned(),
            RuleAtom::Constant(c) => Ok(c.domain_id()),
        }
    }
}
