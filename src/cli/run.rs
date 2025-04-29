use super::config::Config;
use crate::lang::{
    dynamics::{Denotation, Knowledge},
    *,
};
use std::collections::HashSet;

pub fn stdin_to_string() -> Result<String, std::io::Error> {
    use std::io::Read as _;
    let mut buffer = String::new();
    std::io::stdin().lock().read_to_string(&mut buffer)?;
    Ok(buffer)
}

pub fn run_check(
    config: Config,
    source: String,
    w: &mut impl std::io::Write,
) -> Result<Denotation<Knowledge>, String> {
    let source = preprocessing::comments_removed(source);
    if config.test("source") {
        let _ = writeln!(w, "source after preprocessing: <<\n{}\n>>", &source);
    }
    let mut parse_result = parse::all_consuming(parse::program)(&source);
    let program = match &mut parse_result {
        Err(nom::Err::Error(e)) => {
            let x = nom::error::convert_error(source.as_str(), e.clone());
            return Err(x);
        }
        Err(e) => return Err(format!("parse error: {:#?}", e)),
        Ok((rest, program)) => {
            assert!(rest.is_empty());
            program
        }
    };
    if let Some(part_name) = program.repeatedly_defined_part() {
        return Err(format!("~ ~ ERROR: repeatedly defined part name: {:?}", part_name));
    }
    if config.test("ast1") {
        let _ = writeln!(w, "ast before preprocessing: {:#?}", program);
    }
    preprocessing::normalize_domain_id_formatting(program, config.test("local"));
    // println!("BEFORE EQ {:#?}", program);
    let eq_classes = EqClasses::new(program);
    if config.test("eq") {
        let _ = writeln!(
            w,
            "domain equivalence class representatives {:?}",
            eq_classes.get_representatives()
        );
        let _ = writeln!(
            w,
            "domain equivalence class representative members {:?}",
            eq_classes.get_representative_members()
        );
    }
    eq_classes.normalize_equal_domain_ids(program);
    // println!("AFTER EQ {:#?}", program);
    if let Err(e) = eq_classes.check_primitives() {
        return Err(format!("domain equivalence class error: {:?}", e));
    }

    preprocessing::deanonymize_variables(program);
    if config.test("save") {
        preprocessing::add_antecedent_variables_as_pos_literals(program);
    }
    if config.test("ast2") {
        let _ = writeln!(w, "ast after preprocessing: {:#?}", program);
    }
    let dumn = program.depended_undefined_names().collect::<HashSet<_>>();
    if !dumn.is_empty() {
        let _ = writeln!(w, "~ ~ WARNING: dependend undefined parts: {:?} ~ ~", dumn);
    }
    let ep = program.executable(config.executable_config());
    if config.test("ir") {
        let _ = writeln!(w, "internal representation: {:#?}", ep);
    }
    let ep = match ep {
        Err(e) => return Err(format!("~ ~ ERROR: error constructing executable: {:#?}", e)),
        Ok(ep) => ep,
    };
    // println!("EP {:#?}", ep);
    if let Some(cycle) = ep.unbounded_domain_cycle() {
        return Err(format!(
            "~ ~ ERROR: termination uncertain due to unbounded domain cycle: {:?} ~ ~",
            cycle
        ));
    }
    if !ep.used_undeclared.is_empty() {
        let _ = writeln!(
            w,
            "~ ~ WARNING: domains undeclared but are variables or have arguements: {:?} ~ ~",
            ep.used_undeclared
        );
    }
    let pug = program.part_usage_graph();
    // println!("{:#?}", pug);
    let seal_breaks = pug.iter_breaks(&ep).collect::<HashSet<_>>();
    if !seal_breaks.is_empty() {
        let _ = writeln!(w, "~ ~ WARNING: seal breaks: {:#?} ~ ~", seal_breaks);
    }
    let denotation_res = ep.denotation();
    if config.test("how") {
        let _ = writeln!(w, "how: {:#?}", ep.how(&denotation_res));
    }
    let denotation = denotation_res.denotation;
    if !config.test("no-deno") {
        let _ = if config.test("cluster") {
            writeln!(w, "denotation: {:#?}", denotation)
        } else {
            writeln!(w, "denotation: {:#?}", denotation.bare())
        };
    }
    Ok(denotation)
}
