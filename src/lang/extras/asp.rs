use crate::*;
use std::collections::{HashMap, HashSet};

type VarRewrites<'a> = HashMap<&'a VariableId, RuleAtom>;

impl RuleAtom {
    fn asp_rewrite(&self, var_rewrites: &VarRewrites) -> Self {
        match self {
            Self::Variable(vid) => {
                if let Some(ra) = var_rewrites.get(vid) {
                    return ra.clone();
                }
            }
            Self::Construct { did, args } => {
                if did.is_primitive() {
                    return self.clone();
                }
                return Self::Construct {
                    did: did.clone(),
                    args: args.iter().map(|ra| ra.asp_rewrite(var_rewrites)).collect(),
                };
            }
            Self::Constant(c) => {
                return RuleAtom::Construct { did: c.domain_id().clone(), args: vec![self.clone()] }
            }
        }
        self.clone()
    }
}
impl RuleLiteral {
    fn asp_rewrite(&self, var_rewrites: &VarRewrites) -> Self {
        Self { sign: self.sign.clone(), ra: self.ra.asp_rewrite(var_rewrites) }
    }
}
impl AnnotatedRule {
    fn asp_rewrite(&self, dd: &DomainDefinitions) -> Option<Self> {
        let root_vars = self.rule.root_vars().collect::<HashSet<&VariableId>>();

        let mut uninstantiable = false;
        let var_rewrites: HashMap<&VariableId, RuleAtom> = root_vars
            .into_iter()
            .filter_map(|vid| {
                let did = self.v2d.get(vid).expect("must");
                let num_params = if did.is_primitive() {
                    1
                } else if let Some(params) = dd.get(did) {
                    params.len()
                } else {
                    // this is all pointless
                    uninstantiable = true;
                    return None;
                };
                let args = (0..num_params)
                    .map(|i| RuleAtom::Variable(VariableId(format!("{:?}{}", vid, i))))
                    .collect();
                let ra = RuleAtom::Construct { did: did.clone(), args };
                Some((vid, ra))
            })
            .collect();
        if uninstantiable {
            None
        } else {
            Some(Self {
                v2d: self.v2d.clone(),
                rule: Rule {
                    consequents: self
                        .rule
                        .consequents
                        .iter()
                        .map(|ra| ra.asp_rewrite(&var_rewrites))
                        .collect(),
                    antecedents: self
                        .rule
                        .antecedents
                        .iter()
                        .map(|ra| ra.asp_rewrite(&var_rewrites))
                        .collect(),
                },
            })
        }
    }
}
impl ExecutableProgram {
    pub fn asp_print(&self) -> String {
        self.asp_print_inner().expect("string write cannot fail")
    }
    fn asp_print_inner(&self) -> Result<String, std::fmt::Error> {
        let mut s = String::default();
        use std::fmt::Write;

        // inference rules
        for ar in &self.annotated_rules {
            for rule in ar.rule.split_consequents() {
                let ar = AnnotatedRule { rule, v2d: ar.v2d.clone() };
                if let Some(ar) = ar.asp_rewrite(&self.dd) {
                    write!(&mut s, "{:?}\n", ar.rule)?;
                }
            }
        }
        write!(&mut s, "\n")?;

        // constraints
        for did in &self.emissive {
            if let Some(_) = self.dd.get(did) {
                let vid = VariableId("V".into());
                let v2d: VariableTypes = Some((vid.clone(), did.clone())).into_iter().collect();
                let rule = Rule {
                    consequents: vec![],
                    antecedents: vec![RuleLiteral { sign: Sign::Pos, ra: RuleAtom::Variable(vid) }],
                };
                if let Some(ar) = (AnnotatedRule { v2d, rule }).asp_rewrite(&self.dd) {
                    write!(&mut s, "{:?}\n", ar.rule)?;
                }
            }
        }
        write!(&mut s, "\n")?;

        // choices
        for (did, params) in self.dd.iter() {
            if !self.is_sealed(did) {
                let mut v2d = VariableTypes::default();
                let mut args = vec![];
                let mut antecedents = vec![];
                for (i, param) in params.iter().enumerate() {
                    let vid = VariableId(format!("V{}", i));
                    let ra = RuleAtom::Variable(vid.clone());
                    let rl = RuleLiteral { sign: Sign::Pos, ra: ra.clone() };
                    v2d.insert(vid, param.clone());
                    args.push(ra);
                    antecedents.push(rl);
                }
                let consequents = vec![RuleAtom::Construct { did: did.clone(), args }];
                let ar = AnnotatedRule { v2d, rule: Rule { consequents, antecedents } }
                    .asp_rewrite(&self.dd);
                if let Some(mut ar) = ar {
                    let consequent = ar.rule.consequents.pop().unwrap();
                    let consequent_new = {
                        match consequent.clone() {
                            RuleAtom::Construct { did, args } => RuleAtom::Construct {
                                did: DomainId(format!("__new_{:?}", did)),
                                args,
                            },
                            _ => unreachable!(),
                        }
                    };
                    write!(&mut s, "0{{ {:?} }}1{:?}\n", &consequent_new, ar.rule)?;
                    write!(&mut s, "{:?} :- {:?}.\n", &consequent, &consequent_new)?;
                }
            }
        }
        write!(&mut s, "\n")?;

        Ok(s.replace("!", "not "))
    }
}
