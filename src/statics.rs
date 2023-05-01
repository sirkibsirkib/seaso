use crate::ast::*;

use std::collections::{HashMap, HashSet};

type ParamsMap = HashMap<DomainId, (StatementIdx, Vec<DomainId>)>;

type FirstSealedAt = HashMap<DomainId, StatementIdx>;
pub type RuleToVidToDid = HashMap<StatementIdx, VidToDid>;
pub type VidToDid = HashMap<VariableId, DomainId>;

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
}

#[derive(Debug)]
pub struct SealBreak {
    pub sealed: StatementIdx,
    pub modified: StatementIdx,
    pub did: DomainId,
}

impl Program {
    pub fn emitted(&self) -> HashSet<&DomainId> {
        self.statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Emit { did } => Some(did),
                _ => None,
            })
            .collect()
    }
    pub fn check(&self) -> Result<RuleToVidToDid, (&Statement, CheckErr)> {
        self.check_inner().map_err(|e| (&self.statements[e.sidx], e))
    }
    pub fn check_inner(&self) -> Result<RuleToVidToDid, CheckErr> {
        // step 1: no duplicate domain definitions
        let pm = self.new_params_map()?;

        // step 2: all constructs are declared
        let declarations = self.declarations();
        for (sidx, statement) in self.statements.iter().enumerate() {
            if let Some(did) = statement.undeclared_domain_id(&declarations) {
                return Err(CheckErr { sidx, err: StmtCheckErr::UndefinedConstructor { did } });
            }
        }

        // step 3: check rules and return variable types
        self.statements
            .iter()
            .enumerate()
            .flat_map(|(sidx, statement)| {
                statement
                    .rule_typing(&pm)
                    .map(|x| x.map(|x| (sidx, x)).map_err(|err| CheckErr { sidx, err }))
            })
            .collect()
    }

    pub fn seal_break(&self) -> Option<SealBreak> {
        let mut sfa = FirstSealedAt::default();
        for (sidx, statement) in self.statements.iter().enumerate() {
            match statement {
                Statement::Seal { did } => {
                    sfa.insert(did.clone(), sidx);
                }
                Statement::Rule(Rule { consequents, .. }) => {
                    for consequent in consequents {
                        if let RuleAtom::Construct { did, .. } = consequent {
                            if let Some(&s_sidx) = sfa.get(did) {
                                return Some(SealBreak {
                                    sealed: s_sidx,
                                    modified: sidx,
                                    did: did.clone(),
                                });
                            }
                        }
                    }
                }
                Statement::Emit { did } => {
                    if let Some(&s_sidx) = sfa.get(did) {
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

    fn new_params_map(&self) -> Result<ParamsMap, CheckErr> {
        let mut pm = ParamsMap::default();
        for (sidx, statement) in self.statements.iter().enumerate() {
            if let Statement::Defn { did, params } = statement {
                let value = (sidx, params.clone());
                if let Some((previous_sidx, previous_params)) =
                    pm.insert(did.clone(), value.clone())
                {
                    if &previous_params != params {
                        return Err(CheckErr {
                            sidx,
                            err: StmtCheckErr::ConflictingDefinition {
                                previous_sidx,
                                did: did.clone(),
                            },
                        });
                    }
                }
            }
        }
        Ok(pm)
    }
    fn declarations(&self) -> HashSet<DomainId> {
        self.statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Decl { did } | Statement::Defn { did, .. } => Some(did.clone()),
                _ => None,
            })
            .collect()
    }
}

impl DomainId {
    fn is_primitive(&self) -> bool {
        match self.0.as_ref() {
            "int" | "str" => true,
            _ => false,
        }
    }
}

impl Statement {
    fn undeclared_domain_id(&self, declarations: &HashSet<DomainId>) -> Option<DomainId> {
        match self {
            Statement::Decl { .. } | Statement::Defn { .. } => None,
            Statement::Emit { did } | Statement::Seal { did } => {
                if !declarations.contains(did) {
                    Some(did.clone())
                } else {
                    None
                }
            }
            Statement::Rule(Rule { consequents, antecedents }) => consequents
                .iter()
                .chain(antecedents.iter().map(|antecedent| &antecedent.ra))
                .map(|ra| ra.undeclared_domain_id(declarations))
                .next()
                .flatten(),
        }
    }
    fn rule_typing(&self, pm: &ParamsMap) -> Option<Result<VidToDid, StmtCheckErr>> {
        match self {
            Statement::Rule(Rule { consequents, antecedents }) => {
                let mut v2d = VidToDid::default();
                let mut vids = Default::default();
                for ra in consequents.iter().chain(antecedents.iter().map(|lit| &lit.ra)) {
                    ra.variables(&mut vids);
                    if let Err(err) = ra.typing(pm, &mut v2d) {
                        return Some(Err(err));
                    }
                }
                Some(if let Some(vid) = vids.iter().find(|&vid| !v2d.contains_key(vid)) {
                    Err(StmtCheckErr::NoTypes { vid: vid.clone() })
                } else {
                    // enumerability check
                    let mut enumerable = HashSet::default();
                    for antecedent in antecedents {
                        antecedent.variables_if_enumerable(&v2d, &mut enumerable);
                    }
                    for consequent in consequents.iter() {
                        let did = consequent.domain_id(&v2d).unwrap();
                        if did.is_primitive() {
                            return Some(Err(StmtCheckErr::PrimitiveConsequent {
                                did: did.clone(),
                            }));
                        }
                    }
                    for antecedent in antecedents.iter() {
                        let did = antecedent.ra.domain_id(&v2d).unwrap();
                        if did.is_primitive() {
                            return Some(Err(StmtCheckErr::PrimitiveAntecedent {
                                did: did.clone(),
                            }));
                        }
                    }
                    if let Some(vid) = vids.difference(&enumerable).next() {
                        Err(StmtCheckErr::VariableNotEnumerable { vid: vid.clone() })
                    } else {
                        Ok(v2d)
                    }
                })
            }
            _ => None,
        }
    }
}

impl Constant {
    fn did(&self) -> DomainId {
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
    fn apparent_did(&self) -> Option<DomainId> {
        match self {
            RuleAtom::Construct { did, .. } => Some(did.clone()),
            RuleAtom::Constant { c } => Some(c.did()),
            RuleAtom::Variable { .. } => None,
        }
    }
    fn typing(&self, pm: &ParamsMap, v2d: &mut VidToDid) -> Result<(), StmtCheckErr> {
        match self {
            RuleAtom::Construct { did, args } => {
                let (_defn_sidx, param_dids) =
                    pm.get(did).ok_or(StmtCheckErr::UndefinedConstructor { did: did.clone() })?;
                if param_dids.len() != args.len() {
                    return Err(StmtCheckErr::WrongArity {
                        did: did.clone(),
                        param_count: param_dids.len(),
                        args: args.len(),
                    });
                }
                for (arg, param_did) in args.iter().zip(param_dids.iter()) {
                    if let RuleAtom::Variable { vid } = arg {
                        match v2d.insert(vid.clone(), param_did.clone()) {
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
                    arg.typing(pm, v2d)?
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn variables(&self, vids: &mut HashSet<VariableId>) {
        match self {
            RuleAtom::Constant { .. } => {}
            RuleAtom::Variable { vid } => drop(vids.insert(vid.clone())),
            RuleAtom::Construct { args, .. } => {
                for arg in args {
                    arg.variables(vids)
                }
            }
        }
    }
    fn undeclared_domain_id(&self, declarations: &HashSet<DomainId>) -> Option<DomainId> {
        match self {
            RuleAtom::Construct { did, args } => {
                if !declarations.contains(did) {
                    return Some(did.clone());
                }
                args.iter().map(|ra| ra.undeclared_domain_id(declarations)).next().flatten()
            }
            _ => None,
        }
    }
    // NOTE: assumes `v2d` covers all occurring variables
    pub fn domain_id<'a: 'c, 'b: 'c, 'c>(&'a self, v2d: &'b VidToDid) -> Option<&'c DomainId> {
        match self {
            RuleAtom::Construct { did, .. } => Some(did),
            RuleAtom::Variable { vid } => v2d.get(vid),
            _ => None,
        }
    }
}

impl RuleLiteral {
    pub fn is_enumerable_in<'a: 'c, 'b: 'c, 'c>(
        &'a self,
        v2d: &'b VidToDid,
    ) -> Option<&'c DomainId> {
        if self.sign == Sign::Pos {
            let did = match &self.ra {
                RuleAtom::Construct { did, .. } => did,
                RuleAtom::Variable { vid } => v2d.get(vid).expect("Checked before, I think"),
                _ => return None,
            };
            if !did.is_primitive() {
                return Some(did);
            }
        };
        None
    }
    // NOTE: assumes `v2d` covers all occurring variables
    fn variables_if_enumerable(&self, v2d: &VidToDid, variables: &mut HashSet<VariableId>) {
        if self.sign == Sign::Pos {
            match &self.ra {
                RuleAtom::Construct { did, args } if !did.is_primitive() => {
                    for arg in args {
                        arg.variables(variables)
                    }
                }
                RuleAtom::Variable { vid } => {
                    let did = v2d.get(vid).expect("Checked before, I think");
                    if !did.is_primitive() {
                        self.ra.variables(variables)
                    }
                }
                _ => {}
            }
        }
    }
}
