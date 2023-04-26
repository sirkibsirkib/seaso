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

struct Typing {
    map: HashMap<(StatementIdx, VariableId), DomainId>,
}
enum CheckErr {
    ParamsMapErr(ParamsMapErr),
    UndeclaredDomainId { did: DomainId },
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
        // step 1: unique definitions
        let pm = self.new_params_map().map_err(CheckErr::ParamsMapErr)?;

        // step 2: all constructs are declared
        for (sid, statement) in self.statements.iter().enumerate() {
            if let Some(did) = self.undeclared_domain_id() {
                return Err(CheckErr::UndeclaredDomainId { did });
            }
        }

        // Ok(Info { pm, rule_infos })
        todo!()
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
}

impl RuleAtom {
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

// impl Statement {
//     fn rule_info(&self, pm: &ParamsMap) -> RuleInfo {
//         let mut ri = RuleInfo::default();
//         match self {
//             Statement::Rule { consequents, antecedents } => {
//                     ra.analyze_types(pm, ctx, &mut ri, false)
//                 }
//                 for RuleLiteral { sign, ra } in antecedents {
//                     match sign {
//                         Sign::Pos => {}
//                         Sign::Neg => {}
//                     }
//                 }
//             }
//             _ => {}
//         }
//         ri
//     }
// }
// impl RuleInfo {
//     fn add_domain_mapping(&mut self, ra: RuleAtom, did: DomainId) {
//         self.ra_domains.entry(ra).or_default().insert(did);
//     }
// }
// impl DomainId {
//     fn int() -> Self {
//         Self("int".into())
//     }
//     fn str() -> Self {
//         Self("str".into())
//     }
// }
// impl RuleAtom {
//     fn primitive(&self) -> bool {}
//     fn analyze_types(&self, pm: &ParamsMap, ri: &mut RuleInfo, within_enumerable_atom: bool) {
//         match self {
//             RuleAtom::Var { vid } => ri.variables.insert(vid.clone()),
//             RuleAtom::IntConst { c } => ri.add_domain_mapping(self.clone(), DomainId::int()),
//             RuleAtom::StrConst { c } => ri.add_domain_mapping(self.clone(), DomainId::str()),
//             RuleAtom::Construct { did, args } => {
//                 ri.add_domain_mapping(self.clone(), did.clone());
//             }
//         }
//     }
// }
