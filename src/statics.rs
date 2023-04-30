use crate::ast::*;
use core::ops::Range;
use std::collections::{HashMap, HashSet};

// enum DeconstructionErr {
//     UndefinedConstructor { did: DomainId },
//     ArgsParamsArityMismatch { did: DomainId, args: usize, params: usize },
// }

type ParamsMap = HashMap<DomainId, (StatementIdx, Vec<DomainId>)>;

#[derive(Debug)]
pub enum ParamsMapErr {
    DuplicateDefn([StatementIdx; 2]),
}

type FirstSealedAt = HashMap<DomainId, StatementIdx>;
pub type RuleToVidToDid = HashMap<StatementIdx, VidToDid>;
pub type VidToDid = HashMap<VariableId, DomainId>;

#[derive(Debug)]
pub enum CheckErr {
    ParamsMapErr(ParamsMapErr),
    UndefinedConstructor { sidx: StatementIdx, did: DomainId },
    VidToDidErr { sidx: StatementIdx, err: VidToDidErr },
}
#[derive(Debug)]
pub enum VidToDidErr {
    UndefinedConstructor { did: DomainId },
    MultipleTypes { vid: VariableId, domains: [DomainId; 2] },
    NoTypes { vid: VariableId },
    WrongArity { did: DomainId, params: usize, args: usize },
    VariableNotEnumerable { vid: VariableId },
}

trait OnlyIf: Sized {
    fn only_if(self, b: bool) -> Option<Self> {
        if b {
            Some(self)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct SealBreak {
    pub sealed: StatementIdx,
    pub modified: StatementIdx,
}

impl Program {
    pub fn check(&self) -> Result<RuleToVidToDid, CheckErr> {
        // step 1: no duplicate domain definitions
        let pm = self.new_params_map().map_err(CheckErr::ParamsMapErr)?;

        // step 2: all constructs are declared
        let declarations = self.declarations();
        for (sidx, statement) in self.statements.iter().enumerate() {
            if let Some(did) = statement.undeclared_domain_id(&declarations) {
                return Err(CheckErr::UndefinedConstructor { sidx, did });
            }
        }

        // step 3: check rules and return variable types
        self.statements
            .iter()
            .enumerate()
            .flat_map(|(sidx, statement)| {
                statement.rule_typing(&pm).map(|x| {
                    x.map(|x| (sidx, x)).map_err(|err| CheckErr::VidToDidErr { sidx, err })
                })
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
                                return Some(SealBreak { sealed: s_sidx, modified: sidx });
                            }
                        }
                    }
                }
                Statement::Emit { did } => {
                    if let Some(&s_sidx) = sfa.get(did) {
                        return Some(SealBreak { sealed: s_sidx, modified: sidx });
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn new_params_map(&self) -> Result<ParamsMap, ParamsMapErr> {
        let mut pm = ParamsMap::default();
        for (sidx, statement) in self.statements.iter().enumerate() {
            if let Statement::Defn { did, params } = statement {
                let value = (sidx, params.clone());
                if let Some(previous) = pm.insert(did.clone(), value) {
                    return Err(ParamsMapErr::DuplicateDefn([previous.0, sidx]));
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
            "int" | "str" | "eq" => true,
            _ => false,
        }
    }
}

impl Statement {
    fn undeclared_domain_id(&self, declarations: &HashSet<DomainId>) -> Option<DomainId> {
        match self {
            Statement::Decl { .. } | Statement::Defn { .. } => None,
            Statement::Emit { did } | Statement::Seal { did } => {
                if declarations.contains(did) {
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
    fn rule_typing(&self, pm: &ParamsMap) -> Option<Result<VidToDid, VidToDidErr>> {
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
                    Err(VidToDidErr::NoTypes { vid: vid.clone() })
                } else {
                    // enumerability check
                    let mut enumerable = HashSet::default();
                    for antecedent in antecedents {
                        antecedent.variables_if_enumerable(&v2d, &mut enumerable);
                    }
                    if let Some(vid) = vids.difference(&enumerable).next() {
                        Err(VidToDidErr::VariableNotEnumerable { vid: vid.clone() })
                    } else {
                        Ok(v2d)
                    }
                })
            }
            _ => None,
        }
    }
}

impl RuleAtom {
    fn typing(&self, pm: &ParamsMap, v2d: &mut VidToDid) -> Result<(), VidToDidErr> {
        match self {
            RuleAtom::Construct { did, args } => {
                let (_defn_sidx, params) =
                    pm.get(did).ok_or(VidToDidErr::UndefinedConstructor { did: did.clone() })?;
                if params.len() != args.len() {
                    return Err(VidToDidErr::WrongArity {
                        did: did.clone(),
                        params: params.len(),
                        args: args.len(),
                    });
                }
                for (arg, param) in args.iter().zip(params.iter()) {
                    if let RuleAtom::Variable { vid } = arg {
                        match v2d.insert(vid.clone(), param.clone()) {
                            Some(param2) if param != &param2 => {
                                return Err(VidToDidErr::MultipleTypes {
                                    vid: vid.clone(),
                                    domains: [param.clone(), param2.clone()],
                                });
                            }
                            _ => {}
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
