use crate::ast::*;
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

type Typing = HashMap<StatementIdx, RuleTyping>;

#[derive(Debug, Default)]
pub struct RuleTyping {
    map: HashMap<VariableId, DomainId>,
}
#[derive(Debug)]
pub enum CheckErr {
    ParamsMapErr(ParamsMapErr),
    UndefinedConstructor { sidx: StatementIdx, did: DomainId },
    RuleTypingErr { sidx: StatementIdx, err: RuleTypingErr },
}
#[derive(Debug)]
pub enum RuleTypingErr {
    UndefinedConstructor { did: DomainId },
    MultipleTypes { vid: VariableId, domains: [DomainId; 2] },
    NoTypes { vid: VariableId },
    WrongArity { did: DomainId, params: usize, args: usize },
    VariableNotEnumerable { vid: VariableId },
}

#[derive(Debug)]
pub struct SealBreak {
    pub sealed: StatementIdx,
    pub modified: StatementIdx,
}

impl Program {
    pub fn check(&self) -> Result<Typing, CheckErr> {
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
        let t: Typing = self
            .statements
            .iter()
            .enumerate()
            .flat_map(|(sidx, statement)| {
                statement.rule_typing(&pm).map(|x| {
                    x.map(|x| (sidx, x)).map_err(|err| CheckErr::RuleTypingErr { sidx, err })
                })
            })
            .collect::<Result<Typing, _>>()?;

        Ok(t)
    }

    pub fn seal_break(&self) -> Option<SealBreak> {
        let mut sfa = FirstSealedAt::default();
        for (sidx, statement) in self.statements.iter().enumerate() {
            match statement {
                Statement::Seal { did } => {
                    sfa.insert(did.clone(), sidx);
                }
                Statement::Rule { consequents, .. } => {
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
                if declarations.contains(did) {
                    Some(did.clone())
                } else {
                    None
                }
            }
            Statement::Rule { consequents, antecedents } => consequents
                .iter()
                .chain(antecedents.iter().map(|antecedent| &antecedent.ra))
                .map(|ra| ra.undeclared_domain_id(declarations))
                .next()
                .flatten(),
        }
    }
    fn rule_typing(&self, pm: &ParamsMap) -> Option<Result<RuleTyping, RuleTypingErr>> {
        match self {
            Statement::Rule { consequents, antecedents } => {
                let mut rt = RuleTyping::default();
                let mut vids = Default::default();
                for ra in consequents.iter().chain(antecedents.iter().map(|lit| &lit.ra)) {
                    ra.variables(&mut vids);
                    if let Err(err) = ra.typing(pm, &mut rt) {
                        return Some(Err(err));
                    }
                }
                Some(if let Some(vid) = vids.iter().find(|&vid| !rt.map.contains_key(vid)) {
                    Err(RuleTypingErr::NoTypes { vid: vid.clone() })
                } else {
                    // enumerability check
                    let mut enumerable = HashSet::default();
                    for antecedent in antecedents {
                        antecedent.variables_if_enumerable(&rt, &mut enumerable);
                    }
                    if let Some(vid) = vids.difference(&enumerable).next() {
                        Err(RuleTypingErr::VariableNotEnumerable { vid: vid.clone() })
                    } else {
                        Ok(rt)
                    }
                })
            }
            _ => None,
        }
    }
}

impl RuleAtom {
    fn typing(&self, pm: &ParamsMap, rt: &mut RuleTyping) -> Result<(), RuleTypingErr> {
        match self {
            RuleAtom::Construct { did, args } => {
                let (_defn_sidx, params) =
                    pm.get(did).ok_or(RuleTypingErr::UndefinedConstructor { did: did.clone() })?;
                if params.len() != args.len() {
                    return Err(RuleTypingErr::WrongArity {
                        did: did.clone(),
                        params: params.len(),
                        args: args.len(),
                    });
                }
                for (arg, param) in args.iter().zip(params.iter()) {
                    if let RuleAtom::Variable { vid } = arg {
                        match rt.map.insert(vid.clone(), param.clone()) {
                            Some(param2) if param != &param2 => {
                                return Err(RuleTypingErr::MultipleTypes {
                                    vid: vid.clone(),
                                    domains: [param.clone(), param2.clone()],
                                });
                            }
                            _ => {}
                        }
                    }
                }
                for arg in args {
                    arg.typing(pm, rt)?
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn variables(&self, vids: &mut HashSet<VariableId>) {
        match self {
            RuleAtom::IntConst { .. } | RuleAtom::StrConst { .. } => {}
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
}

impl RuleLiteral {
    fn variables_if_enumerable(&self, rt: &RuleTyping, variables: &mut HashSet<VariableId>) {
        if self.sign == Sign::Pos {
            match &self.ra {
                RuleAtom::Construct { did, args } if !did.is_primitive() => {
                    for arg in args {
                        arg.variables(variables)
                    }
                }
                RuleAtom::Variable { vid } => {
                    let did = rt.map.get(vid).expect("Checked before, I think");
                    if !did.is_primitive() {
                        self.ra.variables(variables)
                    }
                }
                _ => {}
            }
        }
    }
}
