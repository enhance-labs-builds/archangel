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
fn verdict_of(rule: &Value, repo: &FixtureRepo) -> Verdict {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**", "other": "other/**" },
        "rules": [rule],
    });
    let world = world(&config, repo);
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
        verdict_of(&refused, &a_imports_lib()),
        refuses("files in src may not import lib"),
        "src/a.ts imports lib, which the rule refuses, so the verdict quotes its sentence"
    );
    let permitted = json!({ "name": "r", "files": "src", "mayNotImport": "other" });
    assert_eq!(
        verdict_of(&permitted, &a_imports_lib()),
        permits("the target is not in other"),
        "the import reaches lib and not other, so a rule against other permits it"
    );
}

#[test]
fn may_only_import_permits_its_list_and_its_own_files() {
    let in_list = json!({ "name": "r", "files": "src", "mayOnlyImport": ["lib"] });
    assert_eq!(
        verdict_of(&in_list, &a_imports_lib()),
        permits("the target is in lib"),
        "lib is on the list, so the verdict names it as what permits the import"
    );

    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "")
        .file("lib/b.ts", "")
        .file("other/c.ts", "");
    let own = json!({ "name": "r", "files": "src", "mayOnlyImport": ["lib"] });
    assert_eq!(
        verdict_of(&own, &repo),
        permits("the target is in src"),
        "a file may always import its own part, though src is not on the list"
    );
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
        permits("the same instance of feature"),
        "a captured part permits its own instance, and the verdict says so, not the list"
    );
}

#[test]
fn may_only_be_imported_by_judges_the_target_and_permits_its_list_and_its_own_files() {
    let by_list = json!({ "name": "r", "files": "lib", "mayOnlyBeImportedBy": ["src"] });
    assert_eq!(
        verdict_of(&by_list, &a_imports_lib()),
        permits("the file is in src"),
        "src is on the list of importers of lib, so the import is permitted"
    );

    let refused = json!({ "name": "r", "files": "lib", "mayOnlyBeImportedBy": ["other"] });
    assert_eq!(
        verdict_of(&refused, &a_imports_lib()),
        refuses("files in lib may only be imported by other"),
        "only other may import lib here, so an import from src is refused"
    );

    let target_elsewhere = json!({ "name": "r", "files": "other", "mayOnlyBeImportedBy": ["src"] });
    assert_eq!(
        verdict_of(&target_elsewhere, &a_imports_lib()),
        skips("the target is not in other"),
        "the rule judges targets in other and this one is in lib, so it skips"
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
    assert_eq!(
        of(&judged, "r").verdict,
        permits("the file is in lib"),
        "a file in lib may import lib, whoever the list names"
    );
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
    assert_eq!(
        of(&inside, "e").verdict,
        permitted,
        "an import inside the instance cart needs no entry file"
    );
    let at_entry = world.judge("src/login/a.ts", 2);
    assert_eq!(
        of(&at_entry, "e").verdict,
        permitted,
        "an import from another instance is permitted when it reaches index.ts"
    );
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
        assert_eq!(verdict_of(&rule, &a_imports_lib()), skips(text), "{rule}");
    }
}

#[test]
fn a_rule_skips_a_file_that_is_not_in_its_files_and_an_import_that_does_not_resolve() {
    let elsewhere = json!({ "name": "r", "files": "other", "mayNotImport": "lib" });
    assert_eq!(
        verdict_of(&elsewhere, &a_imports_lib()),
        skips("the file is not in other"),
        "the rule holds only files in other, so it skips src/a.ts"
    );

    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './gone';\n")
        .file("lib/b.ts", "")
        .file("other/c.ts", "");
    let rule = json!({ "name": "r", "files": "src", "mayNotImport": "lib" });
    assert_eq!(
        verdict_of(&rule, &repo),
        skips("the import does not resolve"),
        "an import with no target cannot be judged, so the rule skips, not refuses"
    );
}

#[test]
fn a_rule_skips_a_kind_of_import_that_it_does_not_count() {
    let rule = json!({ "name": "r", "files": "src", "mayNotImport": "lib", "imports": "types" });
    assert_eq!(
        verdict_of(&rule, &a_imports_lib()),
        skips("the rule does not count this kind of import"),
        "a rule on type imports skips this runtime import and says why"
    );
}

#[test]
fn a_rule_with_an_area_says_which_end_is_outside_it() {
    let importer = json!({
        "name": "r", "files": "src", "mayOnlyImport": ["other"], "within": "other/**",
    });
    assert_eq!(
        verdict_of(&importer, &a_imports_lib()),
        skips("lib/b.ts is outside the area other/**"),
        "the rule judges from src/a.ts, so the end outside the area is the target"
    );
    let target = json!({
        "name": "r", "files": "lib", "mayOnlyBeImportedBy": ["other"], "within": "other/**",
    });
    assert_eq!(
        verdict_of(&target, &a_imports_lib()),
        skips("src/a.ts is outside the area other/**"),
        "the rule judges lib/b.ts, so the end outside the area is the importer"
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
        skips("react is outside the repository"),
        "a row has no verdict on a package, and says that it left the repository"
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
        assert_eq!(verdict_of(&rule, &a_imports_lib()), skips(text), "{rule}");
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
        skips("the manifest does not list the package the way that the rule names"),
        "the rule names dev dependencies and react is a plain one, so the rule skips"
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
        skips("the instance differs from what the rule names"),
        "both files are in the instance cart and the rule names different ones, so it skips"
    );
}

const KINDS: &str = "**/*.{kind}.ts";

/// A row keeps its name in the judgement of a file with another kind, with no kind, and outside the
/// files of its table.
/// A table over `src` with a file of each kind, a file of no kind, and a file outside `src`.
fn a_table_of_files_in_src() -> (Value, FixtureRepo) {
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
    (config, repo)
}

#[test]
fn a_row_says_why_it_skips_a_file() {
    let (config, repo) = a_table_of_files_in_src();
    let world = world(&config, &repo);
    let judged = world.judge("src/a.entity.ts", 1);
    assert_eq!(
        of(&judged, "t.port").verdict,
        skips("the file has the kind entity, not port"),
        "the file has the kind entity, so row port skips it and names both kinds"
    );
    assert_eq!(
        of(&judged, "t.lib").verdict,
        skips("the file is not in lib"),
        "a row key that names a part is no kind"
    );
    assert_eq!(
        of(&judged, "t.entity").verdict,
        permits("the target is in its row"),
        "row entity lists port, and the target b.port.ts has that kind"
    );
    assert_eq!(
        of(&judged, "t.every-file-has-a-kind").verdict,
        skips("this rule checks files, not imports"),
        "the table's own rule has no verdict on any import"
    );

    let no_kind = world.judge("src/plain.ts", 1);
    assert_eq!(
        of(&no_kind, "t.port").verdict,
        skips("the file has no kind in t"),
        "src/plain.ts has no kind, so no row of t applies to it"
    );

    let outside = world.judge("lib/c.entity.ts", 1);
    assert_eq!(
        of(&outside, "t.entity").verdict,
        skips("the file is outside the table's files"),
        "lib/c.entity.ts has a kind but is not in `files: src`, so row entity skips it"
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
        skips("the file is not in gate"),
        "gate names a group and no kind, so the skip speaks of the group and not of kinds"
    );
}

/// `src/cart/a.views.ts` imports, by line: a kind with a row, a kind with no row, its own kind in
/// another instance, the part on its row, a file with no kind or part, and a package.
fn a_view_that_imports_one_of_everything() -> (Value, FixtureRepo) {
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
    (config, repo)
}

#[test]
fn a_refusal_of_a_row_names_the_target_in_the_words_of_its_table() {
    let (config, repo) = a_view_that_imports_one_of_everything();
    let world = world(&config, &repo);
    let refusal =
        |line: u32| match &of(&world.judge("src/cart/a.views.ts", line), "t.views").verdict {
            Refuses(text) => text.clone(),
            other => panic!("line {line}: {other:?}"),
        };
    let text = "views may import only lib, and itself in the same instance";
    assert_eq!(
        refusal(1),
        format!("api is not in its row ({text})"),
        "line 1 reaches b.api, so the refusal names its kind, api"
    );
    assert_eq!(
        refusal(2),
        format!("other is not in its row ({text})"),
        "line 2 reaches c.other, and the refusal names that kind though it has no row"
    );
    assert_eq!(
        refusal(3),
        format!("another instance of views is not in its row ({text})"),
        "line 3 reaches the same kind in another instance, and the refusal says so"
    );
    assert_eq!(
        refusal(5),
        format!("none.ts is not in its row ({text})"),
        "line 5 reaches a file in no part or kind, so the refusal names its path"
    );
    assert_eq!(
        refusal(6),
        format!("pg is not in its row ({text})"),
        "line 6 reaches a package, so the refusal names the package"
    );
    assert_eq!(
        of(&world.judge("src/cart/a.views.ts", 4), "t.views").verdict,
        permits("the target is in its row"),
        "line 4 reaches lib, the one cell of the row, so the row permits it"
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
        refuses("lib is not in its row (views may import only itself)"),
        "lib/b.ts has no kind, so the refusal names its part"
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
        refuses("port may not import itself"),
        "port has no cell, so its refusal mentions no list"
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
    assert_eq!(
        names,
        ["first", "second", "gone"],
        "the judgement lists every rule in config order, the rule that is off included"
    );
    assert!(
        matches!(judged.rules[0].verdict, Refuses(_)),
        "first forbids lib, which a.ts imports: {:?}",
        judged.rules[0].verdict
    );
    assert!(
        matches!(judged.rules[1].verdict, Permits(_)),
        "second forbids other, which a.ts does not import: {:?}",
        judged.rules[1].verdict
    );
    assert_eq!(
        judged.rules[2].verdict,
        skips("the rule is off"),
        "gone would refuse this import, but a rule that is off gives no such verdict"
    );
}

/// `src/cart/a.views.ts` imports a file with a kind in the part `lib`, and then a package.
fn a_view_that_imports_a_part_and_a_package() -> (Value, FixtureRepo) {
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
    (config, repo)
}

#[test]
fn the_ends_of_an_import_name_their_parts_instances_and_kinds() {
    let (config, repo) = a_view_that_imports_a_part_and_a_package();
    let world = world(&config, &repo);
    let judged = world.judge("src/cart/a.views.ts", 1);
    let cart = ("feature".to_owned(), vec!["cart".to_owned()]);
    assert_eq!(
        judged.from.parts,
        [cart],
        "the importer is in part feature with the capture value cart"
    );
    assert_eq!(
        judged.from.kinds,
        [("t".to_owned(), "views".to_owned())],
        "the importer's name gives it the kind views in table t"
    );
    assert_eq!(
        judged.to.parts,
        [("lib".to_owned(), Vec::new())],
        "lib has no capture, so the target's part carries no values"
    );
    assert_eq!(
        judged.to.kinds,
        [("t".to_owned(), "api".to_owned())],
        "the target's name gives it the kind api, though it sits in part lib"
    );
    assert_eq!(
        world.architecture.kinds_of(&world.graph, "lib/b.api.ts"),
        [("t".to_owned(), "api".to_owned())],
        "kinds_of reads the same kind from the path as the end of the import has"
    );

    let package = world.judge("src/cart/a.views.ts", 2);
    assert_eq!(
        package.to.parts,
        [("driver".to_owned(), Vec::new())],
        "a package's part is the externals set that names it"
    );
    assert_eq!(
        package.to.kinds,
        [],
        "a package has no path for a kind pattern to read"
    );
}

#[test]
fn an_unresolved_import_has_no_target() {
    let repo = FixtureRepo::new().file("src/a.ts", "import './gone';\n");
    let config = json!({ "elements": { "src": "src/**" }, "rules": [] });
    let judged = world(&config, &repo).judge("src/a.ts", 1);
    assert_eq!(
        judged.to,
        archangel_rules::ImportEnd::default(),
        "./gone resolves to nothing, so the target has no part, instance or kind"
    );
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
        Some("an exception: the migration".to_owned()),
        "a comment naming rule r permits the import on the next line, with its reason"
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
    assert_eq!(
        of(&judged, "r").permitted_by,
        None,
        "the exception on line 2 covers line 3, so line 1 stays refused"
    );
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

    assert_eq!(
        verdict(&report_of_rule("r", LINE_ONE, "")),
        None,
        "this report has the same rule, file and line, so no baseline permits the import"
    );
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
    assert_eq!(
        of(&judged, "r").permitted_by,
        None,
        "nothing refused this import, so neither exception nor baseline permits it"
    );
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
