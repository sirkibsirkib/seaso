use crate::ast::*;
use std::collections::{HashMap, HashSet};

// pub struct Info {
//     pm: ParamsMap,
//     rule_infos: HashMap<StatementIdx, RuleInfo>,
// }

// #[derive(Default)]
// pub struct RuleInfo {
//     variables: HashSet<VariableId>,
//     ra_domains: HashMap<RuleAtom, HashSet<DomainId>>,
//     failed_desonstructions: Vec<DeconstructionErr>,
// }

enum DeconstructionErr {
    UndefinedConstructor { did: DomainId },
    ArgsParamsArityMismatch { did: DomainId, args: usize, params: usize },
}

// type Typemap = HashMap<TypeMapped, DomainId>;

type ParamsMap = HashMap<DomainId, (StatementIdx, Vec<DomainId>)>;

enum ParamsMapErr {
    DuplicateDefn([StatementIdx; 2]),
}
// enum InfoErr {
//     ParamsMapErr(ParamsMapErr),
//     RuleInfoErr(StatementIdx,RuleInfoErr)
// }
// enum RuleInfoErr {
//     OneAtomTwoDomainIds(RuleAtom, [DomainId; 2]),
// }

type Typing = HashMap<StatementIdx, RuleTyping>;

#[derive(Default)]
struct RuleTyping {
    map: HashMap<VariableId, DomainId>,
}
enum CheckErr {
    ParamsMapErr(ParamsMapErr),
    UndeclaredDomainId { did: DomainId },
    RuleTypingErr { sidx: StatementIdx, err: RuleTypingErr },
}
enum RuleTypingErr {
    MultipleTypes { vid: VariableId, domains: [DomainId; 2] },
    NoTypes { vid: VariableId },
}

impl Program {
    pub fn declared_domain_ids(&self) -> HashSet<DomainId> {
        self.statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Decl { did } | Statement::Defn { did, .. } => Some(did.clone()),
                _ => None,
            })
            .collect()
    }
    pub fn check(&self) -> Result<Typing, CheckErr> {
        // step 1: no duplicate domain definitions
        let pm = self.new_params_map().map_err(CheckErr::ParamsMapErr)?;

        // step 2: all constructs are declared
        for (sid, statement) in self.statements.iter().enumerate() {
            if let Some(did) = self.undeclared_domain_id() {
                return Err(CheckErr::UndeclaredDomainId { did });
            }
        }

        // step 3: unique_types
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

        // step 4: enumerability check
        // TODO

        Ok(t)
    }
    fn new_params_map(&self) -> Result<ParamsMap, ParamsMapErr> {
        let mut pm = ParamsMap::default();
        for (statement_idx, statement) in self.statements.iter().enumerate() {
            if let Statement::Defn { did, params } = statement {
                let value = (statement_idx, params.clone());
                if let Some(previous) = pm.insert(did.clone(), value) {
                    return Err(ParamsMapErr::DuplicateDefn([previous.0, statement_idx]));
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
    fn undeclared_domain_id(&self) -> Option<DomainId> {
        let declarations = self.declarations();
        self.statements
            .iter()
            .map(|statement| statement.undeclared_domain_id(&declarations))
            .next()
            .flatten()
    }
}

#[derive(Clone)]
enum RuleAtomInfoContext {
    ConsequentRoot,
    EnumerableAntecedent,
    NonEnumerableAntecedent,
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
                }
                Some(if let Some(vid) = vids.iter().find(|&vid| !rt.map.contains_key(vid)) {
                    Err(RuleTypingErr::NoTypes { vid: vid.clone() })
                } else {
                    Ok(rt)
                })
            }
            _ => None,
        }
    }
}

impl RuleAtom {
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
