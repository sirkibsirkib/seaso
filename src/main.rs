mod config;
pub mod lang;

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
    let mut parse_result =
        nom::combinator::all_consuming(parse::wsr(parse::parts_and_statements))(&source);
    match &mut parse_result {
        Ok((_, parts)) => {
            if config.test("ast1") {
                println!("ast before preprocessing: {:#?}", parts);
            }
            preprocessing::normalize_domain_id_formatting(parts, config.test("local"));
            let eq_classes = EqClasses::new(parts);
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
            eq_classes.normalize_equal_domain_ids(parts);
            if let Err(e) = eq_classes.check_primitives() {
                println!("domain equivalence class error: {:?}", e);
            } else {
                preprocessing::deanonymize_variables(parts);
                if config.test("save") {
                    preprocessing::add_antecedent_variables_as_pos_literals(parts);
                }
                if config.test("ast2") {
                    println!("ast after preprocessing: {:#?}", parts);
                }
                let part_map_result = statics::PartMap::new(parts.iter());
                match part_map_result {
                    Err(clashing_name) => println!("clashing part name: {:?}", clashing_name),
                    Ok(part_map) => {
                        let dumn = part_map.depended_undefined_names().collect::<HashSet<_>>();
                        println!("dependend undefined parts: {:?}", dumn);
                        let ep = ExecutableProgram::new(&part_map, config.executable_config());
                        if config.test("ir") {
                            println!("internal representation: {:#?}", ep);
                        }
                        match ep {
                            Ok(ep) => {
                                println!("used undeclared domains: {:?}", ep.used_undeclared);
                                let pug = part_map.part_usage_graph();
                                let seal_breaks = pug.iter_breaks(&ep).collect::<HashSet<_>>();
                                println!("seal breaks: {:#?}", seal_breaks);
                                if config.test("asp") {
                                    println!("asp print:\n{}", ep.asp_print());
                                }
                                if config.test("dot") {
                                    println!("ontology dot:\n{}", ep.ontology_dot());
                                }
                                let denotation_res = ep.denotation();
                                if config.test("how") {
                                    println!("how: {:#?}", ep.how(&denotation_res));
                                }
                                if !config.test("no-deno") {
                                    if config.test("bare") {
                                        println!(
                                            "denotation: {:#?}",
                                            denotation_res.denotation.bare()
                                        );
                                    } else {
                                        println!("denotation: {:#?}", denotation_res.denotation);
                                    }
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
