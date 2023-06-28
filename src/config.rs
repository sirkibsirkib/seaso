use std::collections::{HashMap, HashSet};

pub struct Config {
    defined_flag_name_to_description: HashMap<&'static str, &'static str>,
    present_flag_names: HashSet<String>,
}

impl Config {
    pub fn test(&self, flag_name: &'static str) -> bool {
        if !self.defined_flag_name_to_description.contains_key(flag_name) {
            panic!("unknown flag name! `{}`", flag_name);
        }
        self.present_flag_names.contains(flag_name)
    }
}
impl Default for Config {
    fn default() -> Self {
        let defined_flag_name_to_description = [
            ("source", "print given Seaso source code after preprocessing"),
            ("dot", "print GraphViz (.dot) graph ontology of defined domains"),
            ("ast", "print abstract syntax tree after preprocessing"),
            ("asp", "print Clingo-style answer set program solving for facts removing emissions"),
            ("global", "override implicit qualification of part-unqualified domains"),
            ("eq", "print domain equivlance classes and their representative members"),
        ]
        .into_iter()
        .collect();
        if std::env::args().find(|s| s == "--help").is_some() {
            println!("Seaso executor help information. Flags:");
            println!("  --{: <8}  {}", "help", "print this");
            for (k, v) in defined_flag_name_to_description {
                println!("  --{: <8}  {}", k, v);
            }
            std::process::exit(0);
        }
        let present_flag_names = std::env::args()
            .skip(1)
            .filter_map(|mut s| {
                if s.starts_with("--") {
                    s.retain(|c| c != '-');
                    Some(s)
                } else {
                    None
                }
            })
            .collect();
        Self { present_flag_names, defined_flag_name_to_description }
    }
}
