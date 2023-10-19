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

fn main() {
    let config = config::Config::default();
    let source = stdin_to_string().expect("bad stdin");
    let source = preprocessing::comments_removed(source);
    if config.test("source") {
        println!("source after preprocessing: <<\n{}\n>>", &source);
    }
    let mut parse_result = parse::program(&source);
    let program = match &mut parse_result {
        Err(e) => return println!("parse error: {:#?}", e),
        Ok((_, program)) => program,
    };
    if config.test("ast1") {
        println!("ast before preprocessing: {:#?}", program);
    }
    preprocessing::normalize_domain_id_formatting(program, config.test("local"));
    let eq_classes = EqClasses::new(program);
    if config.test("eq") {
        println!("domain equivalence class representatives {:?}", eq_classes.get_representatives());
        println!(
            "domain equivalence class representative members {:?}",
            eq_classes.get_representative_members()
        );
    }
    eq_classes.normalize_equal_domain_ids(program);
    if let Err(e) = eq_classes.check_primitives() {
        return println!("domain equivalence class error: {:?}", e);
    }

    preprocessing::deanonymize_variables(program);
    if config.test("save") {
        preprocessing::add_antecedent_variables_as_pos_literals(program);
    }
    if config.test("ast2") {
        println!("ast after preprocessing: {:#?}", program);
    }
    let part_map_result = statics::PartMap::new(program.parts.iter());
    let part_map = match part_map_result {
        Err(clashing_name) => return println!("clashing part name: {:?}", clashing_name),
        Ok(part_map) => part_map,
    };
    let dumn = part_map.depended_undefined_names().collect::<HashSet<_>>();
    if !dumn.is_empty() {
        println!("~ ~ WARNING: dependend undefined parts: {:?} ~ ~", dumn);
    }
    let ep = ExecutableProgram::new(&part_map, config.executable_config());
    if config.test("ir") {
        println!("internal representation: {:#?}", ep);
    }
    let ep = match ep {
        Err(e) => return println!("~ ~ ERROR: error constructing executable: {:#?}", e),
        Ok(ep) => ep,
    };
    if let Some(cycle) = ep.unbounded_domain_cycle() {
        return println!(
            "~ ~ ERROR: termination uncertain due to unbounded domain cycle: {:?} ~ ~",
            cycle
        );
    }
    if !ep.used_undeclared.is_empty() {
        println!(
            "~ ~ WARNING: domains undeclared but are variables or have arguements: {:?} ~ ~",
            ep.used_undeclared
        );
    }
    let pug = part_map.part_usage_graph();
    let seal_breaks = pug.iter_breaks(&ep).collect::<HashSet<_>>();
    if !seal_breaks.is_empty() {
        println!("~ ~ WARNING: seal breaks: {:#?} ~ ~", seal_breaks);
    }
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
        if config.test("cluster") {
            println!("denotation: {:#?}", denotation_res.denotation)
        } else {
            println!("denotation: {:#?}", denotation_res.denotation.bare())
        }
    }
}
