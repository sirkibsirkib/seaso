use crate::ast::*;

use std::collections::{HashMap, HashSet};

/// Used (internally) to remember where and how constructors are defined.
type DomainDefinitions = HashMap<DomainId, (StatementIdx, Vec<DomainId>)>;

/// These annotate programs to create the internal representation.
/// Ultimnately, they prescribe exactly one type to each variable in each rule.
pub type RuleVariableTypes = HashMap<StatementIdx, VariableTypes>;
pub type VariableTypes = HashMap<VariableId, DomainId>;

#[derive(Debug)]
pub struct CheckErr {
    pub sidx: StatementIdx,
    pub err: StmtCheckErr,
}

#[derive(Debug)]
pub enum StmtCheckErr {
    UndefinedConstructor { did: DomainId },
    OneVariableTwoTypes { vid: VariableId, domains: [DomainId; 2] },
    NoTypes { vid: VariableId },
    WrongArity { did: DomainId, param_count: usize, args: usize },
    VariableNotEnumerable { vid: VariableId },
    PrimitiveConsequent { did: DomainId },
    PrimitiveAntecedent { did: DomainId },
    ConflictingDefinition { did: DomainId, previous_sidx: StatementIdx },
    MistypedArgument { constructor: DomainId, expected: DomainId, got: DomainId },
    DefiningPrimitive { did: DomainId },
}

#[derive(Debug)]
pub struct SealBreak {
    pub sealed: StatementIdx,
    pub modified: StatementIdx,
    pub did: DomainId,
}

impl DomainId {
    const PRIMITIVE_STRS: [&str; 2] = ["str", "int"];
}

impl Program {
    /// Used to filter the truths of the denotation.
    pub fn emitted(&self) -> HashSet<&DomainId> {
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
    pub fn check(&self) -> Result<RuleVariableTypes, (&Statement, CheckErr)> {
        self.check_inner().map_err(|e| (&self.statements[e.sidx], e))
    }
    fn check_inner(&self) -> Result<RuleVariableTypes, CheckErr> {
        let dd = self.domain_definitions()?;
        self.statements
            .iter()
            .enumerate()
            .flat_map(|(sidx, statement)| {
                statement
                    .rule_typing(&dd)
                    .map(|x| x.map(|x| (sidx, x)).map_err(|err| CheckErr { sidx, err }))
            })
            .collect()
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
    fn domain_definitions(&self) -> Result<DomainDefinitions, CheckErr> {
        let mut dd = DomainDefinitions::default();

        let mut f = |sidx, statement: &Statement| {
            if let Statement::Defn { did, params } = statement {
                if did.is_primitive() {
                    return Err(StmtCheckErr::DefiningPrimitive { did: did.clone() });
                }
                let value = (sidx, params.clone());
                if let Some((previous_sidx, previous_params)) =
                    dd.insert(did.clone(), value.clone())
                {
                    if &previous_params != params {
                        return Err(StmtCheckErr::ConflictingDefinition {
                            previous_sidx,
                            did: did.clone(),
                        });
                    }
                }
            }
            Ok(())
        };
        for (sidx, statement) in self.statements.iter().enumerate() {
            f(sidx, statement).map_err(|err| CheckErr { sidx, err })?
        }
        Ok(dd)
    }

    /// Returns the set of all declared domains
    fn declarations(&self) -> HashSet<DomainId> {
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
    fn is_primitive(&self) -> bool {
        Self::PRIMITIVE_STRS.as_slice().contains(&self.0.as_ref())
    }
}

impl Statement {
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
    fn rule_typing(&self, dd: &DomainDefinitions) -> Option<Result<VariableTypes, StmtCheckErr>> {
        match self {
            Statement::Rule(Rule { consequents, antecedents }) => {
                let mut vt = VariableTypes::default();
                let mut vids = HashSet::<VariableId>::default();
                for ra in consequents.iter().chain(antecedents.iter().map(|lit| &lit.ra)) {
                    ra.variables(&mut vids);
                    if let Err(err) = ra.typing(dd, &mut vt) {
                        return Some(Err(err));
                    }
                }
                Some(if let Some(vid) = vids.iter().find(|&vid| !vt.contains_key(vid)) {
                    Err(StmtCheckErr::NoTypes { vid: vid.clone() })
                } else {
                    // enumerability check
                    let mut enumerable = HashSet::default();
                    for antecedent in antecedents {
                        antecedent.variables_if_enumerable(&vt, &mut enumerable);
                    }
                    for consequent in consequents.iter() {
                        let did = consequent.domain_id(&vt).unwrap();
                        if did.is_primitive() {
                            return Some(Err(StmtCheckErr::PrimitiveConsequent {
                                did: did.clone(),
                            }));
                        }
                    }
                    for antecedent in antecedents.iter() {
                        let did = antecedent.ra.domain_id(&vt).unwrap();
                        if did.is_primitive() {
                            return Some(Err(StmtCheckErr::PrimitiveAntecedent {
                                did: did.clone(),
                            }));
                        }
                    }
                    if let Some(vid) = vids.difference(&enumerable).next() {
                        Err(StmtCheckErr::VariableNotEnumerable { vid: vid.clone() })
                    } else {
                        Ok(vt)
                    }
                })
            }
            _ => None,
        }
    }
}

impl Constant {
    fn domain_id(&self) -> DomainId {
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
    // fn occurring_domain_ids<'a>(&'a self, x: &mut HashSet<&'a DomainId>) {
    //     match self {
    //         RuleAtom::Construct { did, args } => {
    //             x.insert(did);
    //             for arg in args {
    //                 arg.occurring_domain_ids(x)
    //             }
    //         }
    //         _ => {}
    //     }
    // }
    fn apparent_did(&self) -> Option<DomainId> {
        match self {
            RuleAtom::Construct { did, .. } => Some(did.clone()),
            RuleAtom::Constant(c) => Some(c.domain_id()),
            RuleAtom::Variable { .. } => None,
        }
    }
    fn typing(&self, dd: &DomainDefinitions, vt: &mut VariableTypes) -> Result<(), StmtCheckErr> {
        match self {
            RuleAtom::Construct { did, args } => {
                let (_defn_sidx, param_dids) =
                    dd.get(did).ok_or(StmtCheckErr::UndefinedConstructor { did: did.clone() })?;
                if param_dids.len() != args.len() {
                    return Err(StmtCheckErr::WrongArity {
                        did: did.clone(),
                        param_count: param_dids.len(),
                        args: args.len(),
                    });
                }
                for (arg, param_did) in args.iter().zip(param_dids.iter()) {
                    if let RuleAtom::Variable(vid) = arg {
                        match vt.insert(vid.clone(), param_did.clone()) {
                            Some(param_did2) if param_did != &param_did2 => {
                                return Err(StmtCheckErr::OneVariableTwoTypes {
                                    vid: vid.clone(),
                                    domains: [param_did.clone(), param_did2.clone()],
                                });
                            }
                            _ => {}
                        }
                    } else if let Some(arg_did) = arg.apparent_did() {
                        if &arg_did != param_did {
                            return Err(StmtCheckErr::MistypedArgument {
                                constructor: did.clone(),
                                expected: param_did.clone(),
                                got: arg_did,
                            });
                        }
                    }
                }
                for arg in args {
                    arg.typing(dd, vt)?
                }
            }
            _ => {}
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
    // NOTE: assumes `vt` covers all occurring variables
    pub fn domain_id<'a: 'c, 'b: 'c, 'c>(&'a self, vt: &'b VariableTypes) -> Option<&'c DomainId> {
        match self {
            RuleAtom::Construct { did, .. } => Some(did),
            RuleAtom::Variable(vid) => vt.get(vid),
            _ => None,
        }
    }
}

impl RuleLiteral {
    pub fn is_enumerable_in<'a: 'c, 'b: 'c, 'c>(
        &'a self,
        vt: &'b VariableTypes,
    ) -> Option<&'c DomainId> {
        if self.sign == Sign::Pos {
            let did = match &self.ra {
                RuleAtom::Construct { did, .. } => did,
                RuleAtom::Variable(vid) => vt.get(vid).expect("Checked before, I think"),
                _ => return None,
            };
            if !did.is_primitive() {
                return Some(did);
            }
        };
        None
    }
    // NOTE: assumes `vt` covers all occurring variables
    fn variables_if_enumerable(&self, vt: &VariableTypes, variables: &mut HashSet<VariableId>) {
        if self.sign == Sign::Pos {
            match &self.ra {
                RuleAtom::Construct { did, args } if !did.is_primitive() => {
                    for arg in args {
                        arg.variables(variables)
                    }
                }
                RuleAtom::Variable(vid) => {
                    let did = vt.get(vid).expect("Checked before, I think");
                    if !did.is_primitive() {
                        self.ra.variables(variables)
                    }
                }
                _ => {}
            }
        }
    }
}
