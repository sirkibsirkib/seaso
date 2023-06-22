use crate::lang::util::snd;
use crate::lang::VecSet;
use crate::*;

use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

pub struct ModuleMap<'a> {
    map: HashMap<&'a ModuleName, &'a Module>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Module {
    pub name: ModuleName,
    pub uses: VecSet<ModuleName>,
    pub statements: VecSet<Statement>,
}
pub struct ModulePreorder<'a> {
    edges: HashSet<[&'a ModuleName; 2]>,
}

/// Identifies which statements first seal and then modify which domain.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct SealBreak<'a> {
    pub did: &'a DomainId,
    pub modifier: &'a ModuleName,
    pub sealer: &'a ModuleName,
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
pub enum ExecutableError<'a> {
    DomainDefinitionsError(DomainDefinitionsError),
    ExecutableRuleError { rule: &'a Rule, err: ExecutableRuleError },
}

//////////////////

impl<'a> ModuleMap<'a> {
    pub fn new(modules: impl IntoIterator<Item = &'a Module>) -> Result<Self, &'a ModuleName> {
        let map =
            util::collect_map_lossless(modules.into_iter().map(|module| (&module.name, module)));
        map.map(|map| Self { map })
    }
    pub fn used_undefined_names(&self) -> impl Iterator<Item = &ModuleName> {
        self.used_modules().filter(|name| !self.map.contains_key(name))
    }
    pub fn used_modules(&self) -> impl Iterator<Item = &ModuleName> {
        self.map.values().flat_map(|module| module.uses.iter())
    }
}

impl<'a> ModulePreorder<'a> {
    pub fn new(module_map: &'a ModuleMap<'a>) -> Self {
        let mut edges = HashSet::<[&'a ModuleName; 2]>::default();
        for (&x, module) in &module_map.map {
            for y in module.uses.iter() {
                if x != y {
                    edges.insert([x, y]);
                }
            }
        }
        for &x in module_map.map.keys() {
            for &y in module_map.map.keys() {
                if x != y {
                    for &z in module_map.map.keys() {
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

impl From<DomainDefinitionsError> for ExecutableError<'_> {
    fn from(e: DomainDefinitionsError) -> Self {
        Self::DomainDefinitionsError(e)
    }
}

impl ExecutableProgram {
    fn module_statements<'a>(
        module_map: &'a ModuleMap<'a>,
    ) -> impl Iterator<Item = (&ModuleName, &Statement)> {
        module_map.map.iter().flat_map(move |(&module_name, module)| {
            module.statements.iter().map(move |statement| (module_name, statement))
        })
    }
    pub fn is_sealed(&self, did: &DomainId) -> bool {
        self.sealers_modifiers.get(did).map(|dsm| !dsm.sealers.is_empty()).unwrap_or(false)
    }
    pub fn new<'a>(module_map: &'a ModuleMap<'a>) -> Result<Self, ExecutableError<'a>> {
        // pass 1: aggregate definitions
        let dd = Self::domain_definitions(Self::module_statements(module_map).map(snd))?;

        let mut annotated_rules = vec![];
        let mut sealers_modifiers = HashMap::<DomainId, DomainSealersModifiers>::default();
        let mut emissive = HashSet::<DomainId>::default();
        let mut declared_undefined = HashSet::<DomainId>::default();
        let mut used_undeclared = HashSet::<DomainId>::default();

        for (module_name, statement) in Self::module_statements(module_map) {
            match statement {
                Statement::Rule(rule) => {
                    rule.occurring_dids(&mut used_undeclared);
                    let v2d = rule
                        .rule_type_variables(&dd)
                        .map_err(|err| ExecutableError::ExecutableRuleError { rule, err })?;
                    for did in rule.consequents.iter().map(|x| x.domain_id(&v2d).expect("WAH")) {
                        sealers_modifiers
                            .entry(did.clone())
                            .or_default()
                            .modifiers
                            .insert(module_name.clone());
                    }
                    annotated_rules.push(AnnotatedRule { v2d, rule: rule.clone() })
                }
                Statement::Emit(did) => {
                    used_undeclared.insert(did.clone());
                    sealers_modifiers
                        .entry(did.clone())
                        .or_default()
                        .modifiers
                        .insert(module_name.clone());
                    emissive.insert(did.clone());
                }
                Statement::Seal(did) => {
                    used_undeclared.insert(did.clone());
                    sealers_modifiers
                        .entry(did.clone())
                        .or_default()
                        .sealers
                        .insert(module_name.clone());
                }
                Statement::Decl(did) => {
                    used_undeclared.insert(did.clone());
                    declared_undefined.insert(did.clone());
                }
                Statement::Defn { did, params } => {
                    used_undeclared.insert(did.clone());
                    for param in params {
                        used_undeclared.insert(param.clone());
                    }
                }
            }
        }
        declared_undefined.retain(|did| dd.contains_key(did));
        used_undeclared.retain(|did| {
            !declared_undefined.contains(did) && !dd.contains_key(did) && !did.is_primitive()
        });
        Ok(ExecutableProgram {
            dd,
            annotated_rules,
            emissive,
            sealers_modifiers,
            declared_undefined,
            used_undeclared,
        })
    }

    pub fn domain_definitions<'a>(
        statements: impl Iterator<Item = &'a Statement>,
    ) -> Result<DomainDefinitions, DomainDefinitionsError> {
        let mut dd = DomainDefinitions::default();
        for statement in statements {
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
}

impl Rule {
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
    fn root_atoms(&self) -> impl Iterator<Item = &RuleAtom> {
        self.consequents.iter().chain(self.antecedents.iter().map(|lit| &lit.ra))
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
