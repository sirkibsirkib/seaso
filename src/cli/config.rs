use crate::lang::ExecutableConfig;
use std::collections::HashSet;

pub struct Config {
    pub present_flag_names: HashSet<String>,
}

impl Config {
    pub fn test(&self, flag_name: &'static str) -> bool {
        if !Self::known_flag_name(flag_name) {
            panic!("unknown flag name! `{}`", flag_name);
        }
        self.present_flag_names.contains(flag_name)
    }
}
static FLAG_DESC_SLICE: &[(&str, &str)] = &[
    ("ast1", "print abstract syntax tree before preprocessing"),
    ("ast2", "print abstract syntax tree after preprocessing"),
    ("cluster", "denotation atoms are shown clustered by domain"),
    ("eq", "print domain equivalence classes and their representative members"),
    ("how", "print the concrete rule antecedents of each truth"),
    ("ir", "print the internal representation (used to compute the denotation)"),
    ("local", "implicitly localize ('namespace') domains to their parts"),
    ("no-deno", "do not print the program denotation, i.e., truths and unknowns"),
    ("source", "print given Seaso source code after preprocessing"),
    ("save", "preprocess rules s.t. they are safe by adding consequent-only variables as positive antecedents"),
    ("sub", "rules implicitly infer all consequents' subconsequents"),
];

impl Config {
    pub fn known_flag_name(s: &str) -> bool {
        FLAG_DESC_SLICE.iter().any(|(key, _)| key == &s)
    }
    pub fn no_flags() -> Self {
        Self { present_flag_names: Default::default() }
    }
    pub fn from_sys_args() -> Self {
        if std::env::args().find(|s| s == "--help").is_some() {
            println!("Seaso executor help information. Flags:");
            println!(" --{: <9}  {}", "help", "print this");
            for (name, desc) in FLAG_DESC_SLICE {
                println!(" --{: <9}  {}", name, desc);
            }
            std::process::exit(0);
        }
        let present_flag_names = std::env::args()
            .skip(1)
            .filter_map(|mut s| {
                if s == "--" {
                    None
                } else if s.starts_with("--") {
                    s.replace_range(0.."--".len(), "");
                    if !Self::known_flag_name(&s) {
                        println!("~ ~ WARNING: unrecognized flag  `{}` ~ ~", s);
                    }
                    Some(s)
                } else {
                    println!("~ ~ WARNING: unrecognized input `{}` ~ ~", s);
                    None
                }
            })
            .collect();
        Self { present_flag_names }
    }
}

impl Config {
    pub fn executable_config(&self) -> ExecutableConfig {
        ExecutableConfig { subconsequence: self.test("sub") }
    }
}
