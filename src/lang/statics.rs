use crate::*;
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

/// Identifies which statements first seal and then modify which domain.
#[derive(Debug)]
pub struct SealBreak {
    pub sealed: StatementIdx,
    pub modified: StatementIdx,
    pub did: DomainId,
}

#[derive(Debug)]
pub enum DomainDefinitionsError {
    ConflictingDefinitions { did: DomainId, params: [Vec<DomainId>; 2] },
    DefiningPrimitive(DomainId),
}

#[derive(Debug)]
pub enum ExecutableRuleError {
    OneVariableTwoTypes { vid: VariableId, domains: [DomainId; 2] },
    MistypedArgument { constructor: DomainId, expected: DomainId, got: DomainId },
    UndefinedConstructor(DomainId),
    VariableNotEnumerable(VariableId),
    WrongArity { did: DomainId, param_count: usize, arg_count: usize },
    NoTypes(VariableId),
}

#[derive(Debug)]
pub enum ExecutableError {
    DomainDefinitionsError(DomainDefinitionsError),
    ExecutableRuleError(ExecutableRuleError),
}

//////////////////

impl DomainId {
    pub const PRIMITIVE_STRS: [&str; 2] = ["str", "int"];
}

impl From<DomainDefinitionsError> for ExecutableError {
    fn from(e: DomainDefinitionsError) -> Self {
        Self::DomainDefinitionsError(e)
    }
}
impl From<ExecutableRuleError> for ExecutableError {
    fn from(e: ExecutableRuleError) -> Self {
        Self::ExecutableRuleError(e)
    }
}
impl Program {
    pub fn domain_definitions(&self) -> Result<DomainDefinitions, DomainDefinitionsError> {
        let mut dd = DomainDefinitions::default();
        for statement in &self.statements {
            if let Statement::Defn { did, params } = statement {
                if did.is_primitive() {
                    return Err(DomainDefinitionsError::DefiningPrimitive(did.clone()));
                }
                let prev = dd.insert(did.clone(), params.clone());
                if let Some(previous_params) = prev {
                    if &previous_params != params {
                        return Err(DomainDefinitionsError::ConflictingDefinitions {
                            did: did.clone(),
                            params: [previous_params, params.clone()],
                        });
                    }
                }
            }
        }
        Ok(dd)
    }
    pub fn executable(&self) -> Result<ExecutableProgram, ExecutableError> {
        let dd = self.domain_definitions()?;
        let mut annotated_rules = vec![];
        let mut emissive = HashSet::<DomainId>::default();
        let mut sealed = HashSet::<DomainId>::default();
        for statement in &self.statements {
            match statement {
                Statement::Rule(rule) => {
                    let v2d = rule.rule_type_variables(&dd)?;
                    annotated_rules.push(AnnotatedRule { v2d, rule: rule.clone() })
                }
                Statement::Emit(did) => {
                    emissive.insert(did.clone());
                }
                Statement::Seal(did) => {
                    sealed.insert(did.clone());
                }
                _ => {}
            }
        }
        Ok(ExecutableProgram { dd, annotated_rules, emissive, sealed })
    }

    pub fn sealed_domains(&self) -> impl Iterator<Item = &DomainId> {
        self.statements.iter().filter_map(|statement| match statement {
            Statement::Seal(did) => Some(did),
            _ => None,
        })
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
}
impl Rule {
    fn rule_type_variables(
        &self,
        dd: &DomainDefinitions,
    ) -> Result<VariableTypes, ExecutableRuleError> {
        let Self { consequents, antecedents } = self;
        let mut vt = VariableTypes::default();
        let mut vids = HashSet::<VariableId>::default();
        for ra in consequents.iter().chain(antecedents.iter().map(|lit| &lit.ra)) {
            ra.variables(&mut vids);
            if let Err(err) = ra.type_variables(dd, &mut vt) {
                return Err(err);
            }
        }
        if let Some(vid) = vids.iter().find(|&vid| !vt.contains_key(vid)) {
            Err(ExecutableRuleError::NoTypes(vid.clone()))
        } else {
            // enumerability check
            let mut enumerable = HashSet::default();
            for antecedent in antecedents {
                if let RuleLiteral { sign: Sign::Pos, ra } = antecedent {
                    ra.variables(&mut enumerable);
                }
            }
            if let Some(vid) = vids.difference(&enumerable).next() {
                Err(ExecutableRuleError::VariableNotEnumerable(vid.clone()))
            } else {
                Ok(vt)
            }
        }
    }
}

static LAZY_INT: OnceLock<DomainId> = OnceLock::new();
static LAZY_STR: OnceLock<DomainId> = OnceLock::new();
impl Constant {
    pub fn domain_id(&self) -> &DomainId {
        match self {
            Self::Int { .. } => LAZY_INT.get_or_init(|| DomainId("int".to_owned())),
            Self::Str { .. } => LAZY_STR.get_or_init(|| DomainId("str".to_owned())),
        }
    }
}

impl RuleAtom {
    pub fn apparent_did(&self) -> Option<&DomainId> {
        match self {
            RuleAtom::Construct { did, .. } => Some(did),
            RuleAtom::Constant(c) => Some(c.domain_id()),
            RuleAtom::Variable { .. } => None,
        }
    }
    fn type_variables(
        &self,
        dd: &DomainDefinitions,
        vt: &mut VariableTypes,
    ) -> Result<(), ExecutableRuleError> {
        if let RuleAtom::Construct { did, args } = self {
            let param_dids =
                dd.get(did).ok_or(ExecutableRuleError::UndefinedConstructor(did.clone()))?;
            if param_dids.len() != args.len() {
                return Err(ExecutableRuleError::WrongArity {
                    did: did.clone(),
                    param_count: param_dids.len(),
                    arg_count: args.len(),
                });
            }
            for (arg, param_did) in args.iter().zip(param_dids.iter()) {
                if let RuleAtom::Variable(vid) = arg {
                    match vt.insert(vid.clone(), param_did.clone()) {
                        Some(param_did2) if param_did != &param_did2 => {
                            return Err(ExecutableRuleError::OneVariableTwoTypes {
                                vid: vid.clone(),
                                domains: [param_did.clone(), param_did2.clone()],
                            });
                        }
                        _ => {}
                    }
                } else if let Some(arg_did) = arg.apparent_did() {
                    if arg_did != param_did {
                        return Err(ExecutableRuleError::MistypedArgument {
                            constructor: did.clone(),
                            expected: param_did.clone(),
                            got: arg_did.clone(),
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
    pub fn domain_id<'a>(&'a self, vt: &'a VariableTypes) -> Result<&'a DomainId, ()> {
        match self {
            RuleAtom::Construct { did, .. } => Ok(did),
            RuleAtom::Variable(vid) => vt.get(vid).ok_or(()),
            RuleAtom::Constant(c) => Ok(c.domain_id()),
        }
    }
}
