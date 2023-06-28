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
static FLAG_DESC_SLICE: &[(&str, &str)] = &[
    ("asp", "print Clingo-style answer set program solving for facts removing emissions"),
    ("ast1", "print abstract syntax tree before preprocessing"),
    ("ast2", "print abstract syntax tree after preprocessing"),
    ("dot", "print GraphViz (.dot) graph ontology of defined domains"),
    ("eq", "print domain equivlance classes and their representative members"),
    ("ir", "print the internal representation (used to compute the denotation)"),
    ("no-local", "do not implicitly localize ('namespace') domains to parts"),
    ("no-deno", "do not print the program denotation, i.e., truths and unknowns"),
    ("source", "print given Seaso source code after preprocessing"),
];
impl Default for Config {
    fn default() -> Self {
        if std::env::args().find(|s| s == "--help").is_some() {
            println!("Seaso executor help information. Flags:");
            println!("  --{: <8}  {}", "help", "print this");
            for (name, desc) in FLAG_DESC_SLICE {
                println!(" --{: <8}  {}", name, desc);
            }
            std::process::exit(0);
        }
        let defined_flag_name_to_description: HashMap<_, _> =
            FLAG_DESC_SLICE.into_iter().copied().collect();
        let present_flag_names = std::env::args()
            .skip(1)
            .filter_map(|mut s| {
                if s == "--" {
                    None
                } else if s.starts_with("--") {
                    s.replace_range(0.."--".len(), "");
                    if !defined_flag_name_to_description.contains_key(s.as_str()) {
                        println!("Warning: unrecognized flag  `{}`", s);
                    }
                    Some(s)
                } else {
                    println!("Warning: unrecognized input `{}`", s);
                    None
                }
            })
            .collect();
        Self { present_flag_names, defined_flag_name_to_description }
    }
}
