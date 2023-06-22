use crate::*;
impl ExecutableProgram {
    pub fn ontology_dot(&self) -> String {
        self.ontology_dot_inner().expect("String write cannot fail")
    }
    fn ontology_dot_inner(&self) -> Result<String, std::fmt::Error> {
        let mut s = String::default();
        use std::fmt::Write as _;
        write!(&mut s, "digraph {{\n")?;
        write!(&mut s, "  node [shape=rect, height=0.1, color=\"red\"];\n")?;
        write!(&mut s, "  edge [];\n")?;
        for did in &self.declared_undefined {
            write!(&mut s, "  {:?} [color=\"blue\"];\n", did)?;
        }
        for (did, params) in &self.dd {
            write!(&mut s, "  {:?} [color=\"green\"];\n", did)?;
            for param in params {
                write!(s, "    {:?} -> {:?};\n", did, param)?;
            }
        }
        for ar in &self.annotated_rules {
            for consequent in &ar.rule.consequents {
                let did_c = consequent.domain_id(&ar.v2d).expect("whee");
                for antecedent in &ar.rule.antecedents {
                    let did_a = antecedent.ra.domain_id(&ar.v2d).expect("whee");
                    let color_str = match antecedent.sign {
                        Sign::Pos => "green",
                        Sign::Neg => "orange",
                    };
                    write!(s, "    {:?} -> {:?} [color={:?}];\n", did_c, did_a, color_str)?;
                }
            }
        }
        write!(&mut s, "}}\n")?;
        Ok(s)
    }
}
