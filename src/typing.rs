use crate::ast::*;
use std::collections::{HashMap, HashSet};

type Typemap = HashMap<TypeMapped, DomainId>;
type Paramsmap = HashMap<DomainId, (StatementIdx, Vec<DomainId>)>;
type Decldefnset = HashSet<DomainId>;

#[derive(PartialEq, Hash, Eq)]
struct TypeMapped {
    statement_idx: StatementIdx,
    ra: RuleAtom,
}

enum DefnmapErr {
    DuplicateDefn([StatementIdx; 2]),
}

impl Program {
    pub fn new_decldefnset(&self) -> Decldefnset {
        self.statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Decl { did } | Statement::Defn { did, .. } => Some(did.clone()),
                _ => None,
            })
            .collect()
    }
    pub fn new_paramsmap(&self) -> Result<Paramsmap, DefnmapErr> {
        let mut pm = Paramsmap::default();
        for (statement_idx, statement) in self.statements.iter().enumerate() {
            if let Statement::Defn { did, params } = statement {
                let value = (statement_idx, params.clone());
                if let Some(previous) = pm.insert(did.clone(), value) {
                    return Err(DefnmapErr::DuplicateDefn([previous.0, statement_idx]));
                }
            }
        }
        Ok(pm)
    }
    pub fn new_typemap(&self) -> Result<Typemap, DefnmapErr> {
        let pm = self.new_paramsmap()?;
        let mut tm = Typemap::default();
        for statement in &self.statements {
            statement.add_type_mappings(&mut tm)
        }
        Ok(tm)
    }
}

impl TypeMapper for Typemap {
    fn map(&mut self, type_mapped: TypeMapped, did: DomainId) {
        drop(self.insert(type_mapped, did))
    }
}

trait TypeMapper {
    fn map(&mut self, type_mapped: TypeMapped, did: DomainId);
}

impl Statement {
    fn add_type_mappings(&self, type_mapper: &mut impl TypeMapper) {
        match self {
            Statement::Rule { consequents, antecedents } => {
                let rule_atoms =
                    consequents.iter().chain(antecedents.iter().map(|literal| &literal.ra));
                for ra in rule_atoms {
                    ra.add_type_mappings(type_mapper)
                }
            }
            _ => {}
        }
    }
}

impl RuleAtom {
    fn add_type_mappings(&self, type_mapper: &mut impl TypeMapper) {
        todo!()
    }
}
