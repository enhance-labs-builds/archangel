//! Scenarios from the command-line spec about `archangel explain <file>:<line>`: the verdict of each
//! rule on one import, and why each other rule skipped it.
#![cfg(test)]

use Verdict::{Permits, Refuses, Skips};
use archangel_config::from_value;
use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Architecture, ImportJudgement, Report, RuleJudgement, Verdict};
use archangel_testkit::{FixtureRepo, table};
use serde_json::{Value, json};

struct World {
    architecture: Architecture,
    graph: ImportGraph,
}

fn world(config: &Value, repo: &FixtureRepo) -> World {
    let config = from_value(config).expect("a valid config");
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default());
    World {
        architecture: Architecture::compile(config),
        graph,
    }
}

impl World {
    fn report(&self) -> Report {
        self.architecture
            .check(&self.graph, None, &mut Work::default())
    }

    fn judge_with(&self, file: &str, line: u32, report: &Report) -> ImportJudgement {
        let imports = &self
            .graph
            .file(file)
            .expect("a file")
            .facts
            .as_ref()
            .expect("facts")
            .imports;
        let index = imports
            .iter()
            .position(|import| import.line == line)
            .expect("an import on the line");
        let resolution = &self.graph.resolutions(file).expect("resolutions")[index];
        self.architecture
            .judge_import(&self.graph, file, &imports[index], resolution, report)
    }

    fn judge(&self, file: &str, line: u32) -> ImportJudgement {
        self.judge_with(file, line, &self.report())
    }
}

fn of<'j>(judged: &'j ImportJudgement, rule: &str) -> &'j RuleJudgement {
    judged
        .rules
        .iter()
        .find(|found| found.rule == rule)
        .unwrap_or_else(|| panic!("no {rule} in {judged:?}"))
}

fn skips(text: &str) -> Verdict {
    Skips(text.to_owned())
}
fn permits(text: &str) -> Verdict {
    Permits(text.to_owned())
}
fn refuses(text: &str) -> Verdict {
    Refuses(text.to_owned())
}

/// The verdict of the only rule of a config on the import on line 1 of `src/a.ts`.
fn verdict_of(rule: &Value, repo: FixtureRepo) -> Verdict {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**", "other": "other/**" },
        "rules": [rule],
    });
    let world = world(&config, &repo);
    let judged = world.judge("src/a.ts", 1);
    judged.rules[0].verdict.clone()
}

/// `src/a.ts` imports `lib/b.ts`, and `other/c.ts` keeps its element.
fn a_imports_lib() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/a.ts", "import '../lib/b';\n")
        .file("lib/b.ts", "")
        .file("other/c.ts", "")
}

#[test]
fn may_not_import_refuses_what_it_names_and_permits_the_rest() {
    let refused = json!({ "name": "r", "files": "src", "mayNotImport": "lib" });
    assert_eq!(
        verdict_of(&refused, a_imports_lib()),
        refuses("files in src may not import lib")
    );
    let permitted = json!({ "name": "r", "files": "src", "mayNotImport": "other" });
    assert_eq!(
        verdict_of(&permitted, a_imports_lib()),
        permits("the target is not in other")
    );
}

#[test]
fn may_only_import_permits_its_list_and_its_own_files() {
    let in_list = json!({ "name": "r", "files": "src", "mayOnlyImport": ["lib"] });
    assert_eq!(
        verdict_of(&in_list, a_imports_lib()),
        permits("the target is in lib")
    );

    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "")
        .file("lib/b.ts", "")
        .file("other/c.ts", "");
    let own = json!({ "name": "r", "files": "src", "mayOnlyImport": ["lib"] });
    assert_eq!(verdict_of(&own, repo), permits("the target is in src"));
}

#[test]
fn may_only_import_on_a_captured_part_says_that_the_instance_is_the_same() {
    let repo = FixtureRepo::new()
        .file("src/cart/a.ts", "import './b';\n")
        .file("src/cart/b.ts", "")
        .file("lib/c.ts", "");
    let config = json!({
        "parts": { "feature": "src/{feature}/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "feature", "mayOnlyImport": ["lib"] }],
    });
    let judged = world(&config, &repo).judge("src/cart/a.ts", 1);
    assert_eq!(
        of(&judged, "r").verdict,
        permits("the same instance of feature")
    );
}

#[test]
fn may_only_be_imported_by_judges_the_target_and_permits_its_list_and_its_own_files() {
    let by_list = json!({ "name": "r", "files": "lib", "mayOnlyBeImportedBy": ["src"] });
    assert_eq!(
        verdict_of(&by_list, a_imports_lib()),
        permits("the file is in src")
    );

    let refused = json!({ "name": "r", "files": "lib", "mayOnlyBeImportedBy": ["other"] });
    assert_eq!(
        verdict_of(&refused, a_imports_lib()),
        refuses("files in lib may only be imported by other")
    );

    let target_elsewhere = json!({ "name": "r", "files": "other", "mayOnlyBeImportedBy": ["src"] });
    assert_eq!(
        verdict_of(&target_elsewhere, a_imports_lib()),
        skips("the target is not in other")
    );

    let repo = FixtureRepo::new()
        .file("lib/a.ts", "import './b';\n")
        .file("lib/b.ts", "")
        .file("src/a.ts", "")
        .file("other/c.ts", "");
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "lib", "mayOnlyBeImportedBy": ["src"] }],
    });
    let judged = world(&config, &repo).judge("lib/a.ts", 1);
    assert_eq!(of(&judged, "r").verdict, permits("the file is in lib"));
}

#[test]
fn an_entry_rule_permits_an_import_in_one_instance_and_refuses_one_past_the_entry() {
    let repo = FixtureRepo::new()
        .file("src/cart/index.ts", "")
        .file("src/cart/a.ts", "import './b';\n")
        .file("src/cart/b.ts", "")
        .file(
            "src/login/a.ts",
            "import '../cart/b';\nimport '../cart/index';\n",
        );
    let config = json!({
        "parts": { "feature": "src/{feature}/**" },
        "rules": [{ "kind": "entry", "name": "e", "element": "feature", "files": ["index.ts"] }],
    });
    let world = world(&config, &repo);
    let inside = world.judge("src/cart/a.ts", 1);
    let permitted = permits("the import stays in one instance, or reaches an entry file");
    assert_eq!(of(&inside, "e").verdict, permitted);
    let at_entry = world.judge("src/login/a.ts", 2);
    assert_eq!(of(&at_entry, "e").verdict, permitted);
    let past = world.judge("src/login/a.ts", 1);
    assert!(matches!(of(&past, "e").verdict, Refuses(_)), "{past:?}");
}

#[test]
fn a_rule_that_judges_no_single_import_says_why() {
    let cases = [
        (
            json!({ "name": "r", "files": "src", "noCycles": true }),
            "noCycles checks cycles, not one import",
        ),
        (
            json!({ "name": "r", "files": "src", "mayNotImport": "lib", "transitively": true }),
            "a chain rule checks chains of imports, not one import",
        ),
        (
            json!({ "name": "r", "files": "src/**", "mustBeNamed": "kebab-case" }),
            "mustBeNamed checks no import",
        ),
        (
            json!({ "name": "r", "files": "other/x/**", "mustNotExist": true }),
            "mustNotExist checks no import",
        ),
    ];
    for (rule, text) in cases {
        assert_eq!(verdict_of(&rule, a_imports_lib()), skips(text), "{rule}");
    }
}

#[test]
fn a_rule_skips_a_file_that_is_not_in_its_files_and_an_import_that_does_not_resolve() {
    let elsewhere = json!({ "name": "r", "files": "other", "mayNotImport": "lib" });
    assert_eq!(
        verdict_of(&elsewhere, a_imports_lib()),
        skips("the file is not in other")
    );

    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './gone';\n")
        .file("lib/b.ts", "")
        .file("other/c.ts", "");
    let rule = json!({ "name": "r", "files": "src", "mayNotImport": "lib" });
    assert_eq!(
        verdict_of(&rule, repo),
        skips("the import does not resolve")
    );
}

#[test]
fn a_rule_skips_a_kind_of_import_that_it_does_not_count() {
    let rule = json!({ "name": "r", "files": "src", "mayNotImport": "lib", "imports": "types" });
    assert_eq!(
        verdict_of(&rule, a_imports_lib()),
        skips("the rule does not count this kind of import")
    );
}

#[test]
fn a_rule_with_an_area_says_which_end_is_outside_it() {
    let importer = json!({
        "name": "r", "files": "src", "mayOnlyImport": ["other"], "within": "other/**",
    });
    assert_eq!(
        verdict_of(&importer, a_imports_lib()),
        skips("lib/b.ts is outside the area other/**")
    );
    let target = json!({
        "name": "r", "files": "lib", "mayOnlyBeImportedBy": ["other"], "within": "other/**",
    });
    assert_eq!(
        verdict_of(&target, a_imports_lib()),
        skips("src/a.ts is outside the area other/**")
    );
}

#[test]
fn a_row_says_that_a_package_is_outside_the_repository() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import 'react';\n")
        .file("node_modules/react/index.js", "")
        .file(".gitignore", "node_modules/\n");
    let config = json!({
        "parts": { "src": "src/**" },
        "rules": [table("t", &json!({ "src": [] }), &json!({}))],
    });
    let judged = world(&config, &repo).judge("src/a.ts", 1);
    assert_eq!(
        of(&judged, "t.src").verdict,
        skips("react is outside the repository")
    );
}

#[test]
fn a_limit_says_which_fact_of_the_import_passes_it_by() {
    let cases = [
        (
            json!({ "names": ["x"] }),
            "the import does not bring in a name that the rule names",
        ),
        (
            json!({ "specifier": "alias" }),
            "the specifier is not written the way that the rule names",
        ),
    ];
    for (limit, text) in cases {
        let mut rule = json!({ "name": "r", "files": "src", "mayNotImport": "lib" });
        for (key, value) in limit.as_object().expect("an object") {
            rule[key] = value.clone();
        }
        assert_eq!(verdict_of(&rule, a_imports_lib()), skips(text), "{rule}");
    }
}

#[test]
fn a_limit_on_a_package_or_an_instance_says_so() {
    let repo = FixtureRepo::new()
        .file(
            "package.json",
            "{ \"name\": \"x\", \"dependencies\": { \"react\": \"1\" } }\n",
        )
        .file("src/a.ts", "import 'react';\n")
        .file(".gitignore", "node_modules/\n")
        .file("node_modules/react/index.js", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "r", "files": "src", "mayNotImport": { "packages": ["react"] }, "dependency": "dev" }],
    });
    let judged = world(&config, &repo).judge("src/a.ts", 1);
    assert_eq!(
        of(&judged, "r").verdict,
        skips("the manifest does not list the package the way that the rule names")
    );

    let repo = FixtureRepo::new()
        .file("src/cart/a.ts", "import './b';\n")
        .file("src/cart/b.ts", "");
    let config = json!({
        "parts": { "feature": "src/{feature}/**" },
        "rules": [{ "name": "r", "files": "feature", "mayNotImport": "feature", "instance": "different" }],
    });
    let judged = world(&config, &repo).judge("src/cart/a.ts", 1);
    assert_eq!(
        of(&judged, "r").verdict,
        skips("the instance differs from what the rule names")
    );
}

const KINDS: &str = "**/*.{kind}.ts";

/// A row keeps its name in the judgement of a file with another kind, with no kind, and outside the
/// files of its table.
#[test]
fn a_row_says_why_it_skips_a_file() {
    let repo = FixtureRepo::new()
        .file("src/a.entity.ts", "import './b.port';\n")
        .file("src/b.port.ts", "")
        .file("src/plain.ts", "import './b.port';\n")
        .file("lib/c.entity.ts", "import './d';\n")
        .file("lib/d.ts", "");
    let config = json!({
        "parts": { "src": "src/**", "lib": "lib/**" },
        "rules": [table(
            "t",
            &json!({ "entity": ["port"], "port": [], "lib": [] }),
            &json!({ "kinds": KINDS, "files": "src" }),
        )],
    });
    let world = world(&config, &repo);
    let judged = world.judge("src/a.entity.ts", 1);
    assert_eq!(
        of(&judged, "t.port").verdict,
        skips("the file has the kind entity, not port")
    );
    assert_eq!(
        of(&judged, "t.lib").verdict,
        skips("the file is not in lib"),
        "a row key that names a part is no kind"
    );
    assert_eq!(
        of(&judged, "t.entity").verdict,
        permits("the target is in its row")
    );
    assert_eq!(
        of(&judged, "t.every-file-has-a-kind").verdict,
        skips("this rule checks files, not imports")
    );

    let no_kind = world.judge("src/plain.ts", 1);
    assert_eq!(
        of(&no_kind, "t.port").verdict,
        skips("the file has no kind in t")
    );

    let outside = world.judge("lib/c.entity.ts", 1);
    assert_eq!(
        of(&outside, "t.entity").verdict,
        skips("the file is outside the table's files")
    );
}

#[test]
fn a_row_key_that_names_a_group_is_no_kind() {
    let repo = FixtureRepo::new()
        .file("src/a.entity.ts", "import './b.gate';\n")
        .file("src/b.gate.ts", "")
        .file("gate/g.ts", "");
    let config = json!({
        "parts": { "src": "src/**" },
        "groups": { "gate": "gate/**" },
        "rules": [table("t", &json!({ "entity": ["gate"], "gate": [] }), &json!({ "kinds": KINDS }))],
    });
    let judged = world(&config, &repo).judge("src/a.entity.ts", 1);
    assert_eq!(
        of(&judged, "t.gate").verdict,
        skips("the file is not in gate")
    );
}

#[test]
fn a_refusal_of_a_row_names_the_target_in_the_words_of_its_table() {
    let repo = FixtureRepo::new()
        .file(
            "src/cart/a.views.ts",
            "import './b.api';\nimport './c.other';\nimport '../login/d.views';\nimport '../../lib/e';\nimport '../../none';\nimport 'pg';\n",
        )
        .file("src/cart/b.api.ts", "")
        .file(".gitignore", "node_modules/\n")
        .file("node_modules/pg/index.js", "")
        .file("src/cart/c.other.ts", "")
        .file("src/login/d.views.ts", "")
        .file("lib/e.ts", "")
        .file("none.ts", "");
    let config = json!({
        "parts": { "lib": "lib/**" },
        "externals": { "driver": ["pg"] },
        "rules": [table(
            "t",
            &json!({ "views": ["lib"], "api": ["driver"] }),
            &json!({ "kinds": "src/{feature}/*.{kind}.ts" }),
        )],
    });
    let world = world(&config, &repo);
    let refusal =
        |line: u32| match &of(&world.judge("src/cart/a.views.ts", line), "t.views").verdict {
            Refuses(text) => text.clone(),
            other => panic!("line {line}: {other:?}"),
        };
    let text = "views may import only lib, and itself in the same instance";
    assert_eq!(refusal(1), format!("api is not in its row ({text})"));
    assert_eq!(refusal(2), format!("other is not in its row ({text})"));
    assert_eq!(
        refusal(3),
        format!("another instance of views is not in its row ({text})")
    );
    assert_eq!(refusal(5), format!("none.ts is not in its row ({text})"));
    assert_eq!(refusal(6), format!("pg is not in its row ({text})"));
    assert_eq!(
        of(&world.judge("src/cart/a.views.ts", 4), "t.views").verdict,
        permits("the target is in its row")
    );
}

#[test]
fn a_refusal_names_the_part_of_a_target_with_no_kind() {
    let repo = FixtureRepo::new()
        .file("src/a.views.ts", "import '../lib/b';\n")
        .file("lib/b.ts", "");
    let config = json!({
        "parts": { "lib": "lib/**", "src": "src/**" },
        "rules": [table("t", &json!({ "views": [] }), &json!({ "kinds": KINDS }))],
    });
    let judged = world(&config, &repo).judge("src/a.views.ts", 1);
    assert_eq!(
        of(&judged, "t.views").verdict,
        refuses("lib is not in its row (views may import only itself)")
    );
}

#[test]
fn a_row_that_may_not_import_itself_says_only_that() {
    let repo = FixtureRepo::new()
        .file("src/a.port.ts", "import './b.port';\n")
        .file("src/b.port.ts", "");
    let config = json!({
        "rules": [table(
            "t",
            &json!({ "port": [] }),
            &json!({ "kinds": KINDS, "mayNotImportItself": ["port"] }),
        )],
    });
    let judged = world(&config, &repo).judge("src/a.port.ts", 1);
    assert_eq!(
        of(&judged, "t.port").verdict,
        refuses("port may not import itself")
    );
}

#[test]
fn each_rule_gets_its_own_verdict_and_a_rule_that_is_off_says_so() {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**", "other": "other/**" },
        "off": ["gone"],
        "rules": [
            { "name": "first", "files": "src", "mayNotImport": "lib" },
            { "name": "second", "files": "src", "mayNotImport": "other" },
            { "name": "gone", "files": "src", "mayNotImport": "lib" },
        ],
    });
    let judged = world(&config, &a_imports_lib()).judge("src/a.ts", 1);
    let names: Vec<&str> = judged.rules.iter().map(|rule| rule.rule.as_str()).collect();
    assert_eq!(names, ["first", "second", "gone"]);
    assert!(matches!(judged.rules[0].verdict, Refuses(_)));
    assert!(matches!(judged.rules[1].verdict, Permits(_)));
    assert_eq!(judged.rules[2].verdict, skips("the rule is off"));
}

#[test]
fn the_ends_of_an_import_name_their_parts_instances_and_kinds() {
    let repo = FixtureRepo::new()
        .file(
            "src/cart/a.views.ts",
            "import '../../lib/b.api';\nimport 'pg';\n",
        )
        .file("lib/b.api.ts", "")
        .file(".gitignore", "node_modules/\n")
        .file("node_modules/pg/index.js", "");
    let config = json!({
        "parts": { "feature": "src/{feature}/**", "lib": "lib/**" },
        "externals": { "driver": ["pg"] },
        "rules": [table("t", &json!({ "views": [] }), &json!({ "kinds": KINDS }))],
    });
    let world = world(&config, &repo);
    let judged = world.judge("src/cart/a.views.ts", 1);
    let cart = ("feature".to_owned(), vec!["cart".to_owned()]);
    assert_eq!(judged.from.parts, [cart]);
    assert_eq!(judged.from.kinds, [("t".to_owned(), "views".to_owned())]);
    assert_eq!(judged.to.parts, [("lib".to_owned(), Vec::new())]);
    assert_eq!(judged.to.kinds, [("t".to_owned(), "api".to_owned())]);
    assert_eq!(
        world.architecture.kinds_of(&world.graph, "lib/b.api.ts"),
        [("t".to_owned(), "api".to_owned())]
    );

    let package = world.judge("src/cart/a.views.ts", 2);
    assert_eq!(package.to.parts, [("driver".to_owned(), Vec::new())]);
    assert_eq!(package.to.kinds, []);
}

#[test]
fn an_unresolved_import_has_no_target() {
    let repo = FixtureRepo::new().file("src/a.ts", "import './gone';\n");
    let config = json!({ "elements": { "src": "src/**" }, "rules": [] });
    let judged = world(&config, &repo).judge("src/a.ts", 1);
    assert_eq!(judged.to, archangel_rules::ImportEnd::default());
}

fn refused_by_exception(comment: &str) -> Option<String> {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            &format!("{comment}\nimport '../lib/b';\nimport '../lib/c';\n"),
        )
        .file("lib/b.ts", "")
        .file("lib/c.ts", "");
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "src", "mayNotImport": "lib" }],
    });
    let judged = world(&config, &repo).judge("src/a.ts", 2);
    of(&judged, "r").permitted_by.clone()
}

#[test]
fn an_exception_permits_only_its_rule_on_its_line() {
    assert_eq!(
        refused_by_exception("// archangel-allow r: the migration"),
        Some("an exception: the migration".to_owned())
    );
    assert_eq!(
        refused_by_exception("// archangel-allow other: the migration"),
        None,
        "an exception for another rule"
    );
    assert_eq!(
        refused_by_exception("// nothing"),
        None,
        "the check reports the violation"
    );
}

#[test]
fn an_exception_for_the_next_import_does_not_permit_this_one() {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import '../lib/b';\n// archangel-allow r: the next one\nimport '../lib/c';\n",
        )
        .file("lib/b.ts", "")
        .file("lib/c.ts", "");
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "src", "mayNotImport": "lib" }],
    });
    let judged = world(&config, &repo).judge("src/a.ts", 1);
    assert_eq!(of(&judged, "r").permitted_by, None);
}

/// The report of a check of `src/a.ts` with this text, and of `src/z.ts` with the other.
fn report_of_rule(rule: &str, a: &str, z: &str) -> Report {
    let repo = FixtureRepo::new()
        .file("src/a.ts", a)
        .file("src/z.ts", z)
        .file("lib/b.ts", "");
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": rule, "files": "src", "mayNotImport": "lib" }],
    });
    world(&config, &repo).report()
}

#[test]
fn only_the_report_of_the_same_rule_file_and_line_says_that_the_check_reports_a_violation() {
    const LINE_ONE: &str = "import '../lib/b';\n";
    const LINE_TWO: &str = "\nimport '../lib/b';\n";
    let repo = FixtureRepo::new()
        .file("src/a.ts", LINE_ONE)
        .file("src/z.ts", "")
        .file("lib/b.ts", "");
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "src", "mayNotImport": "lib" }],
    });
    let world = world(&config, &repo);
    let verdict = |report: &Report| {
        of(&world.judge_with("src/a.ts", 1, report), "r")
            .permitted_by
            .clone()
    };
    let baseline = Some("the baseline".to_owned());

    assert_eq!(verdict(&report_of_rule("r", LINE_ONE, "")), None);
    assert_eq!(
        verdict(&report_of_rule("q", LINE_ONE, "")),
        baseline,
        "another rule"
    );
    assert_eq!(
        verdict(&report_of_rule("r", "", LINE_ONE)),
        baseline,
        "another file"
    );
    assert_eq!(
        verdict(&report_of_rule("r", LINE_TWO, "")),
        baseline,
        "another line"
    );
}

#[test]
fn a_permitted_import_has_nothing_that_permits_it() {
    let rule = json!({
        "elements": { "src": "src/**", "lib": "lib/**", "other": "other/**" },
        "rules": [{ "name": "r", "files": "src", "mayNotImport": "other" }],
    });
    let judged = world(&rule, &a_imports_lib()).judge("src/a.ts", 1);
    assert_eq!(of(&judged, "r").permitted_by, None);
}

fn sentence(config: &Value) -> String {
    world(
        config,
        &FixtureRepo::new()
            .file("src/a.ts", "")
            .file("src/b/c.ts", ""),
    )
    .architecture
    .sentence_text(0, 0)
}

#[test]
fn a_row_reads_as_what_it_may_import() {
    let of_row = |rows: &Value, options: &Value| {
        let mut options = options.clone();
        options["kinds"] = json!("src/{kind}/**");
        sentence(&json!({ "rules": [table("t", rows, &options)] }))
    };
    assert_eq!(
        of_row(&json!({ "a": [] }), &json!({})),
        "a may import only itself"
    );
    assert_eq!(
        of_row(&json!({ "a": [] }), &json!({ "mayNotImportItself": ["a"] })),
        "a may import nothing"
    );
    assert_eq!(
        of_row(
            &json!({ "a": ["b"], "b": [] }),
            &json!({ "mayNotImportItself": ["a"] })
        ),
        "a may import only b"
    );
    assert_eq!(
        of_row(&json!({ "a": ["b"], "b": [] }), &json!({})),
        "a may import only b, and itself"
    );
    assert_eq!(
        of_row(&json!({ "a": [] }), &json!({ "within": "src/**" })),
        "a may import only itself within src/**"
    );
    assert_eq!(
        sentence(&json!({ "rules": [table(
            "t",
            &json!({ "a": [] }),
            &json!({ "kinds": "src/{other}/{kind}/**" }),
        )] })),
        "a may import only itself in the same instance"
    );
}

#[test]
fn a_table_own_rule_says_what_it_covers() {
    let own = |options: &Value, rows: &Value| {
        let config = json!({ "parts": { "src": "src/**" }, "rules": [table("t", rows, options)] });
        let world = world(&config, &FixtureRepo::new().file("src/a.ts", ""));
        let rule = world
            .architecture
            .config()
            .rules
            .iter()
            .position(|rule| rule.name == "t.every-file-has-a-kind")
            .expect("the rule");
        world.architecture.sentence_text(rule, 0)
    };
    assert_eq!(
        own(
            &json!({ "kinds": KINDS, "files": "src" }),
            &json!({ "a": [] })
        ),
        "every file of t has a row"
    );
    assert_eq!(
        own(&json!({ "kinds": KINDS }), &json!({ "a": [] })),
        "a file with a kind has a row; a file with no kind is not checked here"
    );
    assert_eq!(
        own(&json!({}), &json!({ "src": [] })),
        "a file in no row is not checked here; add `files` to check it"
    );
}

#[test]
fn a_limit_reads_as_part_of_the_sentence() {
    let config = json!({
        "parts": { "feature": "src/{feature}/**" },
        "rules": [{
            "name": "r", "files": "feature", "mayNotImport": "feature", "instance": "different",
        }],
    });
    assert_eq!(
        world(&config, &FixtureRepo::new().file("src/a/b.ts", ""))
            .architecture
            .sentence_text(0, 0),
        "files in feature may not import feature in a different instance"
    );
}
