mod lang;

use lang::*;
use std::collections::{HashMap, HashSet};

fn stdin_to_string() -> Result<String, std::io::Error> {
    use std::io::Read as _;
    let mut buffer = String::new();
    std::io::stdin().lock().read_to_string(&mut buffer)?;
    Ok(buffer)
}

struct Config {
    dot: bool,
    asp: bool,
    source: bool,
    denotation: bool,
    ast: bool,
}

impl Config {
    fn new() -> Self {
        let get = |s1| std::env::args().find(|s2| s1 == s2).is_some();
        Self {
            source: get("--source"),
            dot: get("--dot"),
            asp: get("--asp"),
            denotation: get("--denotation"),
            ast: get("--ast"),
        }
    }
}

fn main() -> Result<(), ()> {
    let config = Config::new();
    let source = stdin_to_string().expect("bad stdin");
    let source = preprocessing::comments_removed(source);
    if config.source {
        println!("source after preprocessing: <<\n{}\n>>", &source);
    }
    let mut parse_result = nom::combinator::all_consuming(parse::modules_and_statements)(&source);
    match &mut parse_result {
        Ok((_, modules)) => {
            if config.ast {
                println!("ast: {:#?}", modules);
            }
            match preprocessing::equate_domain_ids(modules) {
                Err(e) => {
                    println!("domain equivalence class error: {:?}", e);
                }
                Ok(eq_classes) => {
                    println!(
                        "domain equivalence class representatives {:?}",
                        eq_classes.iter().collect::<HashMap<_, _>>()
                    );
                    preprocessing::deanonymize_variables(modules);
                    let module_map_result = statics::ModuleMap::new(modules.iter());
                    match module_map_result {
                        Err(clashing_name) => println!("clashing module name: {:?}", clashing_name),
                        Ok(module_map) => {
                            let uumn = module_map.used_undefined_names().collect::<HashSet<_>>();
                            println!("used_undefined_module: {:#?}", uumn);
                            let ep = ExecutableProgram::new(&module_map);
                            match ep {
                                Ok(ep) => {
                                    println!("used undeclared: {:?}", &ep.used_undeclared);
                                    let mp = statics::ModulePreorder::new(&module_map);
                                    let seal_breaks = mp.iter_breaks(&ep).collect::<HashSet<_>>();
                                    println!("seal breaks: {:#?}", &seal_breaks);
                                    if config.asp {
                                        println!("asp print:\n{}", ep.asp_print());
                                    }
                                    if config.dot {
                                        println!("ontology dot:\n{}", ep.ontology_dot());
                                    }
                                    if config.denotation {
                                        println!(
                                            "denotation: {:#?}",
                                            dynamics::Executable::denotation(&ep)
                                        );
                                    }
                                }
                                Err(e) => println!("executable error: {:#?}", e),
                            }
                        }
                    }
                }
            }
        }
        Err(e) => println!("parse error: {:#?}", e),
    }
    Ok(())
}
