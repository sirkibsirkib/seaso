mod config;
mod lang;

use lang::*;
use std::collections::HashSet;

fn stdin_to_string() -> Result<String, std::io::Error> {
    use std::io::Read as _;
    let mut buffer = String::new();
    std::io::stdin().lock().read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn main() -> Result<(), ()> {
    let config = config::Config::default();
    let source = stdin_to_string().expect("bad stdin");
    let source = preprocessing::comments_removed(source);
    if config.test("source") {
        println!("source after preprocessing: <<\n{}\n>>", &source);
    }
    let mut parse_result = nom::combinator::all_consuming(parse::modules_and_statements)(&source);
    match &mut parse_result {
        Ok((_, modules)) => {
            if config.test("ast1") {
                println!("ast before preprocessing: {:#?}", modules);
            }
            preprocessing::normalize_domain_id_formatting(modules, !config.test("no-local"));
            let eq_classes = EqClasses::new(modules);
            if config.test("eq") {
                println!(
                    "domain equivalence class representatives {:?}",
                    eq_classes.get_representatives()
                );
                println!(
                    "domain equivalence class representative members {:?}",
                    eq_classes.get_representative_members()
                );
            }
            eq_classes.normalize_equal_domain_ids(modules);
            if let Err(e) = eq_classes.check_primitives() {
                println!("equivalence class error: {:?}", e);
            } else {
                preprocessing::deanonymize_variables(modules);
                if config.test("ast2") {
                    println!("ast after preprocessing: {:#?}", modules);
                }
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
                                if config.test("asp") {
                                    println!("asp print:\n{}", ep.asp_print());
                                }
                                if config.test("dot") {
                                    println!("ontology dot:\n{}", ep.ontology_dot());
                                }
                                if !config.test("no-deno") {
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
        Err(e) => println!("parse error: {:#?}", e),
    }
    Ok(())
}
