use crate::lang::VecSet;
use crate::*;
use core::hash::Hash;

use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

pub struct PartMap<'a> {
    map: HashMap<&'a PartName, &'a Part>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Part {
    pub name: PartName,
    pub uses: VecSet<PartName>,
    pub statements: VecSet<Statement>,
}
pub struct PartPreorder<'a> {
    edges: HashSet<[&'a PartName; 2]>,
}

/// Identifies which statements first seal and then modify which domain.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct SealBreak<'a> {
    pub did: &'a DomainId,
    pub modifier: &'a PartName,
    pub sealer: &'a PartName,
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
pub enum ExecutableError<'a> {
    ConflictingDefinitions { part_name: &'a PartName, did: DomainId, params: [Vec<DomainId>; 2] },
    DefiningPrimitive { part_name: &'a PartName, did: DomainId, params: &'a [DomainId] },
    ExecutableRuleError { part_name: &'a PartName, rule: &'a Rule, err: ExecutableRuleError },
}

//////////////////

impl<'a> PartMap<'a> {
    pub fn new(parts: impl IntoIterator<Item = &'a Part>) -> Result<Self, &'a PartName> {
        let map = util::collect_map_lossless(parts.into_iter().map(|part| (&part.name, part)));
        map.map(|map| Self { map })
    }
    pub fn depended_undefined_names(&self) -> impl Iterator<Item = &PartName> {
        self.depended_parts().filter(|name| !self.map.contains_key(name))
    }
    pub fn depended_parts(&self) -> impl Iterator<Item = &PartName> {
        self.map.values().flat_map(|part| part.uses.iter())
    }
}

impl<'a> PartPreorder<'a> {
    pub fn new(part_map: &'a PartMap<'a>) -> Self {
        let mut edges = HashSet::<[&'a PartName; 2]>::default();
        for (&x, part) in &part_map.map {
            for y in part.uses.iter() {
                if x != y {
                    edges.insert([x, y]);
                }
            }
        }
        for &x in part_map.map.keys() {
            for &y in part_map.map.keys() {
                if x != y {
                    for &z in part_map.map.keys() {
                        if x != z
                            && y != z
                            && edges.contains(&[x, y])
                            && edges.contains(&[y, z])
                            && !edges.contains(&[x, z])
                        {
                            edges.insert([&x, &z]);
                        }
                    }
                }
            }
        }
        Self { edges }
    }

    pub fn iter_breaks<'b: 'a>(
        &'a self,
        ep: &'b ExecutableProgram,
    ) -> impl Iterator<Item = SealBreak<'a>> + 'a {
        ep.sealers_modifiers.iter().flat_map(move |(did, dsm)| {
            dsm.sealers.iter().flat_map(move |sealer| {
                dsm.modifiers
                    .iter()
                    .filter(move |&modifier| {
                        sealer != modifier && !self.edges.contains(&[sealer, modifier])
                    })
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

impl ExecutableProgram {
    pub fn get_used_undeclared(&self) -> &HashSet<DomainId> {
        &self.used_undeclared
    }
    fn part_statements<'a>(
        part_map: &'a PartMap<'a>,
    ) -> impl Iterator<Item = (&PartName, &Statement)> {
        part_map.map.iter().flat_map(move |(&part_name, part)| {
            part.statements.iter().map(move |statement| (part_name, statement))
        })
    }
    pub fn is_sealed(&self, did: &DomainId) -> bool {
        self.sealers_modifiers.get(did).map(|dsm| !dsm.sealers.is_empty()).unwrap_or(false)
    }
    pub fn new<'a>(
        part_map: &'a PartMap<'a>,
        executable_config: ExecutableConfig,
    ) -> Result<Self, ExecutableError<'a>> {
        // pass 1: domain definitions
        let mut dd = DomainDefinitions::default();
        for (part_name, statement) in Self::part_statements(part_map) {
            match statement {
                Statement::Defn { did, params } => {
                    if did.is_primitive() {
                        return Err(ExecutableError::DefiningPrimitive {
                            part_name,
                            did: did.clone(),
                            params,
                        });
                    }
                    let prev = dd.insert(did.clone(), params.clone());
                    if let Some(previous_params) = prev {
                        if &previous_params != params {
                            return Err(ExecutableError::ConflictingDefinitions {
                                part_name,
                                did: did.clone(),
                                params: [previous_params, params.clone()],
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // pass 2: everything else
        let mut annotated_rules = vec![];
        let mut sealers_modifiers = HashMap::<DomainId, DomainSealersModifiers>::default();
        let mut emissive = HashSet::<DomainId>::default();
        let mut declared = HashSet::<DomainId>::default();
        let mut used = HashSet::<DomainId>::default();
        for (part_name, statement) in Self::part_statements(part_map) {
            match statement {
                Statement::Rule(rule) => {
                    rule.occurring_dids(&mut used);
                    let v2d = rule.rule_type_variables(&dd).map_err(|err| {
                        ExecutableError::ExecutableRuleError { part_name, rule, err }
                    })?;

                    let mut handle_consequent = |ra: &RuleAtom| {
                        if !rule.contains_pos_antecedent(ra, executable_config.subconsequence) {
                            let did = ra.domain_id(&v2d).expect("WAH");
                            sealers_modifiers
                                .entry(did.clone())
                                .or_default()
                                .modifiers
                                .insert(part_name.clone());
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
                    annotated_rules.push(AnnotatedRule { v2d, rule: rule.clone() })
                }
                Statement::Emit(did) => {
                    used.insert(did.clone());
                    emissive.insert(did.clone());
                }
                Statement::Seal(did) => {
                    used.insert(did.clone());
                    sealers_modifiers
                        .entry(did.clone())
                        .or_default()
                        .sealers
                        .insert(part_name.clone());
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
        let used_undeclared =
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
}

impl Rule {
    pub fn consequent_variables(&self) -> HashSet<VariableId> {
        let mut set = HashSet::<VariableId>::default();
        let visitor = &mut |ra: &RuleAtom| {
            if let RuleAtom::Variable(vid) = ra {
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
            if let RuleAtom::Variable(vid2) = ra {
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
            RuleAtom::Variable(vid) => Some(vid),
            _ => None,
        })
    }
    fn occurring_dids(&self, dids: &mut HashSet<DomainId>) {
        for ra in self.root_atoms() {
            ra.occurring_dids(dids)
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
    fn visit_subatoms<'a, 'b>(&'a self, visitor: &'b mut impl FnMut(&'a Self)) {
        visitor(self);
        if let Self::Construct { args, .. } = self {
            for arg in args {
                arg.visit_subatoms(visitor)
            }
        }
    }
    fn occurring_dids(&self, dids: &mut HashSet<DomainId>) {
        if let RuleAtom::Construct { did, args } = self {
            dids.insert(did.clone());
            for arg in args {
                arg.occurring_dids(dids)
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

impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        util::CommaSep { iter: self.consequents.iter(), spaced: true }.fmt(f)?;
        if !self.antecedents.is_empty() {
            write!(f, " :- ")?;
            util::CommaSep { iter: self.antecedents.iter(), spaced: true }.fmt(f)?;
        }
        write!(f, ".")
    }
}
