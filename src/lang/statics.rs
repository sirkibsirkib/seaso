use crate::*;
use core::hash::Hash;

use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

pub type PartUsageGraph<'a> = crate::util::Digraph<&'a PartName>;
pub type ArgumentGraph<'a> = crate::util::Digraph<&'a DomainId>;

/// Identifies which statements first seal and then modify which domain.
#[derive(Eq, Hash, PartialEq)]
pub struct SealBreak<'a> {
    pub did: &'a DomainId,
    pub modifier: &'a StatementAt,
    pub sealer: &'a StatementAt,
}

#[derive(Debug)]
pub enum ExecutableRuleError {
    RepeatedlyDefinedPart { part_name: PartName },
    OneVariableTwoTypes { vid: VariableId, domains: [DomainId; 2] },
    MistypedArgument { constructor: DomainId, expected: DomainId, got: DomainId },
    VariableNotEnumerable(VariableId),
    WrongArity { did: DomainId, param_count: usize, arg_count: usize },
    NoTypes(VariableId),
}

#[derive(Debug)]
pub enum ExecutableError<'a> {
    ConflictingDefinitions { statement_at: StatementAt, did: DomainId, params: [Vec<DomainId>; 2] },
    DefiningPrimitive { statement_at: StatementAt, did: DomainId, params: &'a [DomainId] },
    ExecutableRuleError { statement_at: StatementAt, rule: &'a Rule, err: ExecutableRuleError },
}

//////////////////

impl Program {
    // pub fn parse(source: &str) -> Result<Self, nom::Err<&str>> {
    //     parse::completed(parse::program)(&source).map(|(x, y)| y)
    // }
    pub fn composed(mut self, other: Self) -> Self {
        self.anon_mod_statements.extend(other.anon_mod_statements);
        self.parts.extend(other.parts.into_iter());
        self
    }
    pub fn statements_and_at(&self) -> impl Iterator<Item = (&Statement, StatementAt)> {
        let part_statements = self.parts.iter().flat_map(|part| {
            part.statements
                .iter()
                .zip(std::iter::repeat(StatementAt::InPart { part_name: part.name.clone() }))
        });
        let anon_mod_statements =
            self.anon_mod_statements.iter().enumerate().map(|(statement_index, statement)| {
                (statement, StatementAt::AnonPart { statement_index })
            });
        part_statements.chain(anon_mod_statements)
    }
    pub fn repeatedly_defined_part(&self) -> Option<&PartName> {
        let mut seen_so_far = HashSet::<&PartName>::default();
        for part in self.parts.iter() {
            let name = &part.name;
            if !seen_so_far.insert(name) {
                return Some(name);
            }
        }
        None
    }
    pub fn part_usage_graph(&self) -> PartUsageGraph {
        let mut digraph = PartUsageGraph::default();
        for x in self.parts.iter() {
            digraph.insert_vert(&x.name);
            for y in x.uses.iter() {
                if &x.name != y {
                    digraph.insert_edge([&x.name, y]);
                }
            }
        }
        digraph.transitively_close();
        digraph
    }
    pub fn depended_undefined_names(&self) -> impl Iterator<Item = &PartName> {
        let defined: HashSet<_> = self.parts.iter().map(|part| &part.name).collect();
        self.parts
            .iter()
            .flat_map(|part| part.uses.iter())
            .filter(move |part_name| !defined.contains(part_name))
    }
    pub fn depended_parts(&self) -> impl Iterator<Item = &PartName> {
        self.parts.iter().flat_map(|part| part.uses.iter())
    }

    pub fn executable(
        &self,
        executable_config: ExecutableConfig,
    ) -> Result<ExecutableProgram, ExecutableError> {
        // pass 1: collect domain definitions
        let dd = {
            let mut dd = DomainDefinitions::default();
            for (statement, statement_at) in self.statements_and_at() {
                if let Statement::Defn { did, params } = statement {
                    if did.is_primitive() {
                        return Err(ExecutableError::DefiningPrimitive {
                            statement_at: statement_at.clone(),
                            did: did.clone(),
                            params,
                        });
                    }
                    let prev = dd.insert(did.clone(), params.clone());
                    if let Some(previous_params) = prev {
                        if &previous_params != params {
                            return Err(ExecutableError::ConflictingDefinitions {
                                statement_at,
                                did: did.clone(),
                                params: [previous_params, params.clone()],
                            });
                        }
                    }
                }
            }
            dd
        };

        // pass 2: extract annotated rules from statements. collect usages, sealers, modifiers, ...
        let mut annotated_rules = vec![];
        let mut sealers_modifiers = HashMap::<DomainId, DomainSealersModifiers>::default();
        let mut emissive = HashSet::<DomainId>::default();
        let mut declared = HashSet::<DomainId>::default();
        let mut used = HashSet::<DomainId>::default();

        for (statement, statement_at) in self.statements_and_at() {
            match statement {
                Statement::Rule(rule) => {
                    rule.used_dids(&mut used);
                    let v2d = rule.rule_type_variables(&dd).map_err(|err| {
                        ExecutableError::ExecutableRuleError {
                            statement_at: statement_at.clone(),
                            rule,
                            err,
                        }
                    })?;

                    let mut handle_consequent = |ra: &RuleAtom| {
                        if !rule.contains_pos_antecedent(ra, executable_config.subconsequence) {
                            let did = ra.domain_id(&v2d).expect("WAH");
                            sealers_modifiers
                                .entry(did.clone())
                                .or_default()
                                .modifiers
                                .insert(statement_at.clone());
                        }
                    };
                    for consequent in &rule.consequents {
                        if executable_config.subconsequence {
                            // sub-conseuquents are treated as consequents
                            consequent.visit_subatoms(&mut handle_consequent);
                        } else {
                            handle_consequent(consequent)
                        }
                    }

                    annotated_rules.push(AnnotatedRule {
                        v2d,
                        rule: rule.clone().variable_ascriptions_cleared(),
                    })
                }
                Statement::Emit(did) => {
                    used.insert(did.clone());
                    emissive.insert(did.clone());
                }
                Statement::Seal(did) => {
                    used.insert(did.clone());
                    sealers_modifiers.entry(did.clone()).or_default().sealers.insert(statement_at);
                }
                Statement::Decl(vec) => {
                    for did in vec {
                        declared.insert(did.clone());
                    }
                }
                Statement::Defn { params, did } => {
                    // did a definition pass earlier
                    declared.insert(did.clone());
                    for param in params {
                        used.insert(param.clone());
                    }
                }
            }
        }

        let used_undeclared: HashSet<_> =
            used.into_iter().filter(|did| !declared.contains(did) && !did.is_primitive()).collect();
        let declared_undefined = declared.into_iter().filter(|did| !dd.contains_key(did)).collect();
        Ok(ExecutableProgram {
            dd,
            annotated_rules,
            emissive,
            sealers_modifiers,
            declared_undefined,
            used_undeclared,
            executable_config,
        })
    }

    // pub fn every_mutation(&self) -> impl Iterator<Item = (&PartName, &DomainId)> {
    //     self.parts.iter().flat_map(|part| (&part.name, part.state))
    //     // for part in self.parts.iter() {
    //     //     part.
    //     // }
    // }
}

// impl Statement {
//     fn visit_every_mutation(&self, f: impl FnMut(&DomainId)) {
//         match self {
//             Statement::Rule(rule) => {
//                 for c in rule.consequents.iter() {
//                     match c {}
//                 }
//             }
//             _ => {}
//         }
//     }
// }

impl<'a> PartUsageGraph<'a> {
    fn would_break(&self, sealer: &StatementAt, modifier: &StatementAt) -> bool {
        use StatementAt::{AnonPart, InPart};
        match [sealer, modifier] {
            [InPart { part_name: a }, InPart { part_name: b }] => {
                a != b && !self.contains_edge(&[a, b])
            }
            [AnonPart { statement_index: a }, AnonPart { statement_index: b }] => a < b,
            _ => true,
        }
    }

    pub fn iter_breaks<'b: 'a>(
        &'a self,
        ep: &'b ExecutableProgram,
    ) -> impl Iterator<Item = SealBreak<'a>> + 'a {
        ep.sealers_modifiers.iter().flat_map(move |(did, dsm)| {
            dsm.sealers.iter().flat_map(move |sealer| {
                dsm.modifiers
                    .iter()
                    .filter(move |&modifier| self.would_break(sealer, modifier))
                    .map(move |modifier| SealBreak { sealer, modifier, did })
            })
        })
    }
}

impl DomainId {
    pub const PRIMITIVE_STRS: [&str; 2] = ["str", "int"];
    pub fn is_primitive(&self) -> bool {
        Self::PRIMITIVE_STRS.as_slice().contains(&self.0.as_ref())
    }
}

// adds [X,Y] to argument graph for each X-type construct containing Y-type variable.
fn populate_argument_graph<'a, 'b>(
    ag: &'a mut ArgumentGraph<'b>,
    rule: &'b Rule,
    v2d: &'b VariableTypes,
) {
    fn walk<'a, 'b, 'c>(
        ra: &'a RuleAtom,
        ag: &'b mut ArgumentGraph<'a>,
        v2d: &'a VariableTypes,
        outer_dids: &'c mut Vec<&'a DomainId>,
    ) {
        match ra {
            RuleAtom::Constant(_) => {}
            RuleAtom::Variable { vid, .. } => {
                let inner_did = v2d.get(vid).expect("WAH");
                for &outer_did in outer_dids.iter() {
                    ag.insert_edge([outer_did, inner_did]);
                }
            }
            RuleAtom::Construct { did, args } => {
                outer_dids.push(did);
                for arg in args {
                    walk(arg, ag, v2d, outer_dids);
                }
                outer_dids.pop().unwrap();
            }
        }
    }
    let mut outer_dids = vec![];
    for consequent in rule.consequents.iter() {
        walk(consequent, ag, v2d, &mut outer_dids)
    }
}

impl ExecutableProgram {
    pub fn get_used_undeclared(&self) -> &HashSet<DomainId> {
        &self.used_undeclared
    }
    pub fn is_sealed(&self, did: &DomainId) -> bool {
        self.sealers_modifiers.get(did).map(|dsm| !dsm.sealers.is_empty()).unwrap_or(false)
    }
    pub fn unbounded_domain_cycle(&self) -> Option<&DomainId> {
        // pass 3: (termination detection) build argument graph, throw error on cycle
        let mut ag = ArgumentGraph::default();
        for AnnotatedRule { rule, v2d } in self.annotated_rules.iter() {
            populate_argument_graph(&mut ag, rule, v2d);
        }
        ag.transitively_close();
        for &did in ag.verts().iter() {
            if ag.contains_edge(&[did, did]) {
                return Some(did);
            }
        }
        None
    }
}

impl Rule {
    pub fn consequent_variables(&self) -> HashSet<VariableId> {
        let mut set = HashSet::<VariableId>::default();
        let visitor = &mut |ra: &RuleAtom| {
            if let RuleAtom::Variable { vid, .. } = ra {
                set.insert(vid.clone());
            }
        };
        for consequent in &self.consequents {
            consequent.visit_subatoms(visitor);
        }
        set
    }
    pub fn is_enumerable_variable(&self, vid: &VariableId) -> bool {
        let mut found = false;
        let visitor = &mut |ra: &RuleAtom| {
            if let RuleAtom::Variable { vid: vid2, .. } = ra {
                if vid == vid2 {
                    found = true;
                }
            }
        };

        for antecedent in &self.antecedents {
            if antecedent.sign == Sign::Pos {
                antecedent.ra.visit_subatoms(visitor);
            }
        }
        found
    }
    fn contains_pos_antecedent(&self, ra: &RuleAtom, subconsequence: bool) -> bool {
        let mut pos_antecedent_ras =
            self.antecedents.iter().filter(|x| x.sign == Sign::Pos).map(|x| &x.ra);
        if subconsequence {
            let mut found = false;
            pos_antecedent_ras.any(|ra| {
                ra.visit_subatoms(&mut |ra2| {
                    if ra == ra2 {
                        found = true;
                    }
                });
                found
            })
        } else {
            pos_antecedent_ras.any(|ra2| ra == ra2)
        }
    }
    pub fn split_consequents(&self) -> impl Iterator<Item = Self> + '_ {
        self.consequents.iter().map(|consequent| Self {
            consequents: vec![consequent.clone()],
            antecedents: self.antecedents.clone(),
        })
    }
    pub fn root_vars(&self) -> impl Iterator<Item = &VariableId> {
        self.root_atoms().filter_map(|ra| match ra {
            RuleAtom::Variable { vid, .. } => Some(vid),
            _ => None,
        })
    }
    fn used_dids(&self, dids: &mut HashSet<DomainId>) {
        for ra in self.root_atoms() {
            ra.used_dids(dids)
        }
    }
    pub fn root_atoms(&self) -> impl Iterator<Item = &RuleAtom> {
        self.consequents.iter().chain(self.antecedents.iter().map(|lit| &lit.ra))
    }
    pub fn root_atoms_mut(&mut self) -> impl Iterator<Item = &mut RuleAtom> {
        self.consequents.iter_mut().chain(self.antecedents.iter_mut().map(|lit| &mut lit.ra))
    }
    fn rule_type_variables(
        &self,
        dd: &DomainDefinitions,
    ) -> Result<VariableTypes, ExecutableRuleError> {
        let mut vt = VariableTypes::default();
        let mut vids = HashSet::<VariableId>::default();
        for ra in self.root_atoms() {
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
            for antecedent in &self.antecedents {
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
    fn variable_ascriptions_cleared(mut self) -> Self {
        self.clear_variable_ascriptions();
        self
    }
    fn clear_variable_ascriptions(&mut self) {
        let func = &mut |ra: &mut RuleAtom| {
            if let RuleAtom::Variable { ascription, .. } = ra {
                *ascription = None
            }
        };
        for ra in self.root_atoms_mut() {
            ra.visit_subatoms_mut(func)
        }
    }
}

impl DomainId {
    pub fn int() -> &'static DomainId {
        static LAZY_INT: OnceLock<DomainId> = OnceLock::new();
        LAZY_INT.get_or_init(|| DomainId("int".to_owned()))
    }
    pub fn str() -> &'static DomainId {
        static LAZY_STR: OnceLock<DomainId> = OnceLock::new();
        LAZY_STR.get_or_init(|| DomainId("str".to_owned()))
    }
}
impl Constant {
    pub fn domain_id(&self) -> &'static DomainId {
        match self {
            Self::Int { .. } => DomainId::int(),
            Self::Str { .. } => DomainId::str(),
        }
    }
}

impl RuleAtom {
    fn visit_subatoms(&self, visitor: &mut impl FnMut(&Self)) {
        visitor(self);
        if let Self::Construct { args, .. } = self {
            for arg in args {
                arg.visit_subatoms(visitor)
            }
        }
    }
    fn visit_subatoms_mut(&mut self, visitor: &mut impl FnMut(&mut Self)) {
        visitor(self);
        if let Self::Construct { args, .. } = self {
            for arg in args {
                arg.visit_subatoms_mut(visitor)
            }
        }
    }
    fn used_dids(&self, dids: &mut HashSet<DomainId>) {
        match self {
            RuleAtom::Constant(_) => {}
            RuleAtom::Variable { ascription, .. } => {
                if let Some(did) = ascription {
                    dids.insert(did.clone());
                }
            }
            RuleAtom::Construct { did, args } => {
                if !args.is_empty() {
                    dids.insert(did.clone());
                    for arg in args {
                        arg.used_dids(dids)
                    }
                }
            }
        }
    }
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
        match self {
            Self::Constant(..) => {}
            Self::Variable { vid, ascription } => {
                if let Some(did) = ascription {
                    match vt.insert(vid.clone(), did.clone()) {
                        Some(did2) if did != &did2 => {
                            return Err(ExecutableRuleError::OneVariableTwoTypes {
                                vid: vid.clone(),
                                domains: [did.clone(), did2.clone()],
                            });
                        }
                        _ => {}
                    }
                }
            }
            Self::Construct { did, args } => {
                if let Some(param_dids) = dd.get(did) {
                    if param_dids.len() != args.len() {
                        return Err(ExecutableRuleError::WrongArity {
                            did: did.clone(),
                            param_count: param_dids.len(),
                            arg_count: args.len(),
                        });
                    }
                    for (arg, param_did) in args.iter().zip(param_dids.iter()) {
                        if let Self::Variable { vid, .. } = arg {
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
                }
                // let param_dids =
                //     dd.get(did).ok_or(ExecutableRuleError::UndefinedConstructor(did.clone()))?;

                for arg in args {
                    arg.type_variables(dd, vt)?
                }
            }
        }
        Ok(())
    }
    fn variables(&self, vids: &mut HashSet<VariableId>) {
        match self {
            RuleAtom::Constant { .. } => {}
            RuleAtom::Variable { vid, .. } => drop(vids.insert(vid.clone())),
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
            RuleAtom::Variable { vid, ascription } => {
                ascription.as_ref().or_else(|| vt.get(vid)).ok_or(())
            }
            RuleAtom::Constant(c) => Ok(c.domain_id()),
        }
    }
}

impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        util::CommaSep { iter: self.consequents.iter(), spaced: true }.fmt(f)?;
        if !self.antecedents.is_empty() {
            write!(f, " :- ")?;
        }
        util::CommaSep { iter: self.antecedents.iter(), spaced: true }.fmt(f)
    }
}

impl std::fmt::Debug for SealBreak<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?} broke seal on {:?} in {:?}", self.modifier, self.did, self.sealer)
    }
}

impl std::fmt::Debug for StatementAt {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StatementAt::InPart { part_name } => part_name.fmt(f),
            StatementAt::AnonPart { statement_index } => {
                write!(f, "statement {:?}", statement_index)
            }
        }
    }
}

impl std::fmt::Debug for PartName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "part {:?}", self.0)
    }
}
