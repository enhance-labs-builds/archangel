//! The rule table agrees with a slow reference that reads each condition's definition from the
//! spec directly, for random configs and random pairs of files; each shorthand agrees with the
//! sentences it stands for; and the transitive walk agrees with a search of every chain.
#![cfg(test)]

use archangel_graph::{Import, ImportForm, ImportKind};
use archangel_rules::{Architecture, ReachGraph, Target};
use proptest::prelude::*;
use serde_json::{Value, json};

const ELEMENTS: usize = 4;

/// What a rule side may name beyond [`side_name`]: two externals sets, the second of which also
/// lists what the first lists.
const SIDES: usize = ELEMENTS + 2;

/// e0 and e1 capture a name, e2 and e3 do not.
fn elements() -> Value {
    json!({ "e0": "a/{x}/**", "e1": "b/{x}/**", "e2": "c/**", "e3": "d/**" })
}

fn name(index: usize) -> String {
    format!("e{index}")
}

/// A rule side: an element, or the inline pattern of every instance named `i0`, which also
/// matches the file in no element.
fn side_name(index: usize) -> String {
    if index < ELEMENTS {
        name(index)
    } else {
        "*/i0/**".to_owned()
    }
}

/// The other side of a rule or an area: what `side_name` names, or an externals set.
fn other_side(index: usize) -> String {
    match index {
        5 => "x0".to_owned(),
        6 => "x1".to_owned(),
        index => side_name(index),
    }
}

fn imports() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["all", "runtime", "types"])
}

fn sentence() -> impl Strategy<Value = Value> {
    let condition =
        prop::sample::select(vec!["mayNotImport", "mayOnlyImport", "mayOnlyBeImportedBy"]);
    let instance = prop::sample::select(vec!["", "same", "different"]);
    (
        0..=ELEMENTS,
        condition,
        prop::collection::vec(0..=SIDES, 1..3),
        imports(),
        instance,
        prop::option::of(0..=SIDES),
    )
        .prop_map(|(files, condition, side, imports, instance, area)| {
            let mut rule = json!({
                "files": side_name(files),
                condition: side.into_iter().map(other_side).collect::<Vec<_>>(),
                "imports": imports,
            });
            if let (Some(area), true) = (area, condition != "mayNotImport") {
                rule["within"] = json!(other_side(area));
            }
            // `instance` needs `files` to name an element with a capture.
            if files < 2 && !instance.is_empty() {
                rule["instance"] = json!(instance);
            }
            rule
        })
}

fn shorthand() -> impl Strategy<Value = Value> {
    prop_oneof![
        (prop::collection::vec(0..ELEMENTS, 1..4), imports())
            .prop_map(|(order, imports)| json!({ "kind": "layers", "order": order.into_iter().map(name).collect::<Vec<_>>(), "imports": imports })),
        (0..2usize, imports()).prop_map(|(e, imports)| json!({ "kind": "isolate", "element": name(e), "imports": imports })),
        (0..ELEMENTS, imports()).prop_map(|(e, imports)| json!({ "kind": "entry", "element": name(e), "files": ["index.ts"], "imports": imports })),
    ]
}

fn rule() -> impl Strategy<Value = Value> {
    prop_oneof![sentence(), shorthand()]
}

/// A file: its element, its instance, and whether it is the instance's entry file; `None` is a
/// file in no element.
fn end() -> impl Strategy<Value = Option<(usize, usize, bool)>> {
    prop::option::of((0..ELEMENTS, 0..2usize, any::<bool>()))
}

fn path(end: Option<(usize, usize, bool)>) -> String {
    let Some((element, instance, entry)) = end else {
        return "other/i0/x.ts".to_owned();
    };
    let folder = ["a", "b", "c", "d"][element];
    let file = if entry { "index.ts" } else { "inner/x.ts" };
    if element < 2 {
        format!("{folder}/i{instance}/{file}")
    } else {
        format!("{folder}/{file}")
    }
}

/// One import, as the reference sees it.
struct Case {
    from: Option<(usize, usize, bool)>,
    to: Option<(usize, usize, bool)>,
    /// A package that the import names in place of `to`.
    package: Option<&'static str>,
    kind: ImportKind,
}

impl Case {
    /// Whether the target of the import is on a side.
    fn target_in(&self, side: &str) -> bool {
        match self.package {
            Some(package) => matches!((side, package), ("x0", "pg") | ("x1", "pg" | "zod")),
            None => in_side(side, self.to),
        }
    }

    fn target_in_list(&self, rule: &Value, key: &str) -> bool {
        rule[key].as_array().is_some_and(|sides| {
            sides
                .iter()
                .any(|side| self.target_in(side.as_str().unwrap()))
        })
    }
}

fn element_of(end: Option<(usize, usize, bool)>) -> Option<usize> {
    end.map(|(element, _, _)| element)
}

/// The instance of a file in an element with a capture; a file of e2 or e3 has one instance.
fn instance_of(end: Option<(usize, usize, bool)>) -> Option<(usize, usize)> {
    end.map(|(element, instance, _)| (element, if element < 2 { instance } else { 0 }))
}

fn in_side(side: &str, end: Option<(usize, usize, bool)>) -> bool {
    match side {
        "*/i0/**" => end.is_none_or(|(element, instance, _)| element < 2 && instance == 0),
        element => element_of(end).is_some_and(|e| name(e) == element),
    }
}

fn in_list(rule: &Value, key: &str, end: Option<(usize, usize, bool)>) -> bool {
    rule[key].as_array().is_some_and(|sides| {
        sides
            .iter()
            .any(|side| in_side(side.as_str().unwrap(), end))
    })
}

/// Each condition's and each shorthand's definition, read from the spec.
fn breaks(rule: &Value, case: &Case) -> bool {
    let checked = match rule["imports"].as_str().unwrap() {
        "all" => true,
        "runtime" => case.kind == ImportKind::Runtime,
        _ => case.kind == ImportKind::Type,
    };
    checked && breaks_definition(rule, case)
}

fn breaks_definition(rule: &Value, case: &Case) -> bool {
    let (fe, te) = (element_of(case.from), element_of(case.to));
    let same_instance =
        instance_of(case.from).is_some() && instance_of(case.from) == instance_of(case.to);
    match rule["kind"].as_str() {
        Some("layers") => {
            let order: Vec<&str> = rule["order"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let rank = |e: Option<usize>| e.and_then(|e| order.iter().position(|o| *o == name(e)));
            matches!((rank(fe), rank(te)), (Some(f), Some(t)) if t < f)
        }
        Some("isolate") => {
            let element = rule["element"].as_str().unwrap();
            fe.is_some_and(|f| name(f) == element)
                && te.is_some_and(|t| name(t) == element)
                && !same_instance
        }
        Some(_) => {
            let element = rule["element"].as_str().unwrap();
            let entry = case.to.is_some_and(|(_, _, entry)| entry);
            te.is_some_and(|t| name(t) == element) && !same_instance && !entry
        }
        None => sentence_breaks(rule, case, same_instance),
    }
}

fn sentence_breaks(rule: &Value, case: &Case, same_instance: bool) -> bool {
    let files = rule["files"].as_str().unwrap();
    let (from_in, to_in) = (in_side(files, case.from), case.target_in(files));
    let instance_holds = match rule["instance"].as_str() {
        None => true,
        Some(wanted) => {
            let both = element_of(case.from).is_some_and(|f| name(f) == files)
                && element_of(case.to).is_some_and(|t| name(t) == files);
            both && (wanted == "same") == same_instance
        }
    };
    let area = rule.get("within").map(|area| area.as_str().unwrap());
    let condition = if rule.get("mayNotImport").is_some() {
        from_in && case.target_in_list(rule, "mayNotImport")
    } else if rule.get("mayOnlyImport").is_some() {
        // Its own files are the files of the same instance when `files` is an element with a capture.
        let captured = files == name(0) || files == name(1);
        from_in
            && area.is_none_or(|area| case.target_in(area))
            && !case.target_in_list(rule, "mayOnlyImport")
            && !(to_in && (!captured || same_instance))
    } else {
        to_in
            && area.is_none_or(|area| in_side(area, case.from))
            && !in_list(rule, "mayOnlyBeImportedBy", case.from)
            && !from_in
    };
    condition && instance_holds
}

fn import(kind: ImportKind) -> Import {
    Import {
        specifier: "./x".to_owned(),
        kind,
        form: ImportForm::Static,
        line: 1,
        statement_line: 1,
        column: 1,
        span: (0, 0),
        utf16: (0, 0),
        names: vec!["x".to_owned()],
        names_typed: false,
    }
}

fn compile(rules: &[Value]) -> Architecture {
    let named: Vec<Value> = rules
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            let mut rule = rule.clone();
            rule["name"] = json!(format!("r{i}"));
            rule
        })
        .collect();
    let config = archangel_config::parse_json(
        &json!({
            "elements": elements(),
            "externals": { "x0": ["pg"], "x1": ["pg", "zod"] },
            "rules": named,
        })
        .to_string(),
    )
    .expect("a valid config");
    Architecture::compile(config)
}

/// The rules an import breaks, in config order.
fn broken(architecture: &Architecture, case: &Case) -> Vec<usize> {
    let (from, to) = (path(case.from), path(case.to));
    let target = case.package.map_or(Target::File(&to), Target::Package);
    let mut found = architecture.broken_rules(&from, target, &import(case.kind));
    found.sort_unstable();
    found
}

/// The sentences a shorthand stands for, written by hand.
fn sentences_of(rule: &Value) -> Vec<Value> {
    let imports = rule["imports"].clone();
    match rule["kind"].as_str() {
        Some("layers") => {
            let mut order: Vec<String> = Vec::new();
            for element in rule["order"].as_array().unwrap() {
                let element = element.as_str().unwrap().to_owned();
                if !order.contains(&element) {
                    order.push(element);
                }
            }
            (1..order.len())
                .map(|later| json!({ "files": order[later], "mayNotImport": order[..later], "imports": imports }))
                .collect()
        }
        _ => vec![
            json!({ "files": rule["element"], "mayNotImport": rule["element"], "instance": "different", "imports": imports }),
        ],
    }
}

fn layers_or_isolate() -> impl Strategy<Value = Value> {
    prop_oneof![
        (prop::collection::vec(0..ELEMENTS, 1..5), imports())
            .prop_map(|(order, imports)| json!({ "kind": "layers", "order": order.into_iter().map(name).collect::<Vec<_>>(), "imports": imports })),
        (0..2usize, imports()).prop_map(|(e, imports)| json!({ "kind": "isolate", "element": name(e), "imports": imports })),
    ]
}

/// A graph's node count, which nodes are targets, which are stops, and its imports.
type Graph = (usize, Vec<bool>, Vec<bool>, Vec<(usize, usize)>);

/// A random graph of up to seven nodes in path order, some of them targets and some stops, with
/// its imports.
fn reach_graph() -> impl Strategy<Value = Graph> {
    (2..8usize).prop_flat_map(|count| {
        (
            Just(count),
            prop::collection::vec(any::<bool>(), count),
            prop::collection::vec(prop::bool::weighted(0.2), count),
            prop::collection::vec((0..count, 0..count), 0..count * 3),
        )
    })
}

/// The shortest chain from `source` to a target, the first in path order among chains of one
/// length, by trying every path: a chain never passes through a target or a stop, and never visits
/// a file twice except to end at a source that is itself a target.
fn reference_chain(
    source: usize,
    targets: &[bool],
    stops: &[bool],
    edges: &[(usize, usize)],
) -> Option<Vec<usize>> {
    if stops[source] {
        return None;
    }
    let mut best: Option<Vec<usize>> = None;
    let mut stack = vec![vec![source]];
    while let Some(chain) = stack.pop() {
        let last = *chain.last().unwrap();
        for &(from, to) in edges {
            if from != last || (chain.contains(&to) && !targets[to]) {
                continue;
            }
            let mut longer = chain.clone();
            longer.push(to);
            if targets[to] {
                let better = best
                    .as_ref()
                    .is_none_or(|b| (longer.len(), &longer) < (b.len(), b));
                if better {
                    best = Some(longer);
                }
            } else if !stops[to] {
                stack.push(longer);
            }
        }
    }
    best
}

/// The names a table in the property may use: the part with a capture, the part without one, a
/// group, and two kinds.
const ROW_KEYS: [&str; 5] = ["e0", "e2", "g0", "k0", "k1"];

/// A name in a hand-written rule: a kind is the pattern of its files.
fn hand_name(key: &str) -> String {
    match key {
        "k0" | "k1" => format!("**/*.{key}.ts"),
        other => other.to_owned(),
    }
}

/// A random table: for each row key that it has, its cells; the cells of every row; the rows that
/// may not import themselves; and an area.
#[derive(Debug, Clone)]
struct RandomTable {
    rows: Vec<(usize, Vec<usize>)>,
    every: Vec<usize>,
    itself: Vec<usize>,
    area: Option<usize>,
}

/// The cells: the row keys, an externals set, and a pattern.
const CELLS: [&str; 7] = ["e0", "e2", "g0", "k0", "k1", "x0", "other/**"];

fn random_table() -> impl Strategy<Value = RandomTable> {
    (
        prop::collection::btree_map(
            0..ROW_KEYS.len(),
            prop::collection::vec(0..CELLS.len(), 0..3),
            1..5,
        ),
        prop::collection::vec(0..CELLS.len(), 0..2),
        prop::collection::vec(0..ROW_KEYS.len(), 0..2),
        prop::option::of(prop::sample::select(vec![1_usize, 2])),
    )
        .prop_map(|(rows, every, itself, area)| {
            let keys: Vec<usize> = rows.keys().copied().collect();
            // A kind is a cell only when it is a row, and any other cell that names nothing is refused.
            let known = |cell: &usize| {
                !matches!(CELLS[*cell], "k0" | "k1")
                    || keys.iter().any(|key| ROW_KEYS[*key] == CELLS[*cell])
            };
            let rows: Vec<(usize, Vec<usize>)> = rows
                .into_iter()
                .map(|(key, cells)| (key, cells.into_iter().filter(|cell| known(cell)).collect()))
                .collect();
            let every: Vec<usize> = every.into_iter().filter(|cell| known(cell)).collect();
            // A row in `mayNotImportItself` that names itself is refused, so those lists stay apart.
            let itself: Vec<usize> = itself
                .into_iter()
                .filter(|key| {
                    rows.iter().any(|(row, cells)| {
                        row == key && !cells.iter().any(|cell| CELLS[*cell] == ROW_KEYS[*key])
                    })
                })
                .collect();
            RandomTable {
                rows,
                every,
                itself,
                area,
            }
        })
}

fn cell_names(cells: &[usize]) -> Vec<&'static str> {
    cells.iter().map(|cell| CELLS[*cell]).collect()
}

/// The area of a row as a hand-written rule: the table's area, or every file and the externals
/// sets that the table names.
fn hand_area(table: &RandomTable) -> Value {
    if let Some(area) = table.area {
        return json!(CELLS[area]);
    }
    let names_set = table
        .every
        .iter()
        .chain(table.rows.iter().flat_map(|(_, cells)| cells))
        .any(|cell| CELLS[*cell] == "x0");
    if names_set {
        json!(["**", "x0"])
    } else {
        json!("**")
    }
}

fn table_config(table: &RandomTable) -> Value {
    let rows: serde_json::Map<String, Value> = table
        .rows
        .iter()
        .map(|(key, cells)| (ROW_KEYS[*key].to_owned(), json!(cell_names(cells))))
        .collect();
    let itself: Vec<&str> = table.itself.iter().map(|key| ROW_KEYS[*key]).collect();
    let mut written = json!({
        "kind": "mayImport",
        "name": "t",
        "rows": rows,
        "kinds": "**/*.{kind}.ts",
        "everyRowMayImport": cell_names(&table.every),
        "mayNotImportItself": itself,
    });
    if let Some(area) = table.area {
        written["within"] = json!(CELLS[area]);
    }
    table_elements(&[written])
}

/// The rules that the design says a table means, written by hand.
fn hand_config(table: &RandomTable) -> Value {
    let area = hand_area(table);
    let cell_text = |cell: &str| {
        if matches!(cell, "k0" | "k1") {
            hand_name(cell)
        } else {
            cell.to_owned()
        }
    };
    let rules: Vec<Value> = table
        .rows
        .iter()
        .map(|(key, cells)| {
            let key = ROW_KEYS[*key];
            let mut side: Vec<String> = cell_names(cells)
                .into_iter()
                .chain(cell_names(&table.every))
                .map(cell_text)
                .collect();
            let captured = key == "e0";
            let refused = table.itself.iter().any(|other| ROW_KEYS[*other] == key);
            // A row without a capture permits its own name everywhere, and so does a sentence's list.
            if refused || !captured {
                side.push(hand_name(key));
            }
            let only = json!({ "files": hand_name(key), "mayOnlyImport": side, "within": area });
            if refused {
                let in_area = table.area.map_or_else(
                    || json!(hand_name(key)),
                    |area| json!({ "all": [hand_name(key), CELLS[area]] }),
                );
                let refusal = json!({ "files": hand_name(key), "mayNotImport": in_area });
                json!({ "name": format!("t.{key}"), "sentences": [only, refusal] })
            } else {
                let mut only = only;
                only["name"] = json!(format!("t.{key}"));
                only
            }
        })
        .collect();
    table_elements(&rules)
}

fn table_elements(rules: &[Value]) -> Value {
    json!({
        "parts": { "e0": "a/{x}/**", "e1": "b/{x}/**", "e2": "c/**" },
        "groups": { "g0": "*/i0/**" },
        "externals": { "x0": ["pg"] },
        "rules": rules,
    })
}

/// What an import in the table property breaks, by rule name, with no diagnostic but its rows.
fn broken_names(config: &Value, from: &str, target: Target<'_>) -> Vec<String> {
    let parsed = archangel_config::parse_json(&config.to_string()).expect("a valid config");
    let architecture = Architecture::compile(parsed);
    let mut names: Vec<String> = architecture
        .broken_rules(from, target, &import(ImportKind::Runtime))
        .into_iter()
        .map(|rule| architecture.config().rules[rule].name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

const TABLE_FILES: [&str; 8] = [
    "a/i0/f.k0.ts",
    "a/i0/g.ts",
    "a/i1/f.k1.ts",
    "b/i0/f.ts",
    "c/i0/f.k0.ts",
    "c/h.ts",
    "other/f.ts",
    "plain.k1.ts",
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn violations_agree_with_the_reference_evaluator(
        rules in prop::collection::vec(rule(), 1..6),
        from in end(),
        to in end(),
        package in prop::option::of(prop::sample::select(vec!["pg", "zod", "other"])),
        typed in any::<bool>(),
    ) {
        let architecture = compile(&rules);
        let to = if package.is_some() { None } else { to };
        let case = Case { from, to, package, kind: if typed { ImportKind::Type } else { ImportKind::Runtime } };
        let expected: Vec<usize> = rules.iter().enumerate()
            .filter(|(_, rule)| breaks(rule, &case))
            .map(|(index, _)| index)
            .collect();
        prop_assert_eq!(broken(&architecture, &case), expected);
    }

    #[test]
    fn a_table_agrees_with_one_rule_per_row(
        table in random_table(),
        from in prop::sample::select(TABLE_FILES.to_vec()),
        to in prop::sample::select(TABLE_FILES.to_vec()),
        package in prop::option::of(prop::sample::select(vec!["pg", "zod"])),
    ) {
        let target = package.map_or(Target::File(to), Target::Package);
        prop_assert_eq!(
            broken_names(&table_config(&table), from, target),
            broken_names(&hand_config(&table), from, target)
        );
    }

    #[test]
    fn an_area_agrees_with_its_double_negative(
        files in 2..=ELEMENTS,
        side in prop::collection::vec(0..=ELEMENTS, 1..3),
        area in 0..=ELEMENTS,
        from in end(),
        to in end(),
    ) {
        let (files, area) = (side_name(files), side_name(area));
        let side: Vec<String> = side.into_iter().map(side_name).collect();
        let with_area = json!({
            "files": files, "mayOnlyImport": side, "within": area, "imports": "all",
        });
        let double_negative = json!({
            "files": files,
            "mayNotImport": { "all": [area, { "not": { "any": side.iter().chain([&files]).collect::<Vec<_>>() } }] },
            "imports": "all",
        });
        let case = Case { from, to, package: None, kind: ImportKind::Runtime };
        prop_assert_eq!(
            broken(&compile(&[with_area]), &case),
            broken(&compile(&[double_negative]), &case)
        );
    }

    #[test]
    fn shorthands_agree_with_their_sentences(
        shorthand in layers_or_isolate(),
        from in end(),
        to in end(),
        typed in any::<bool>(),
    ) {
        let sentences = sentences_of(&shorthand);
        let case = Case { from, to, package: None, kind: if typed { ImportKind::Type } else { ImportKind::Runtime } };
        let by_shorthand = !broken(&compile(&[shorthand]), &case).is_empty();
        let by_sentences = !sentences.is_empty() && !broken(&compile(&sentences), &case).is_empty();
        prop_assert_eq!(by_shorthand, by_sentences);
    }

    #[test]
    fn transitive_violations_agree_with_the_reference_search(
        (count, targets, stops, edges) in reach_graph(),
    ) {
        let mut graph = ReachGraph::with_nodes(count);
        for (node, (&target, &stop)) in targets.iter().zip(&stops).enumerate() {
            if target {
                graph.set_target(node);
            }
            if stop {
                graph.set_stop(node);
            }
        }
        for &(from, to) in &edges {
            graph.add_import(from, to);
        }
        let sources: Vec<usize> = (0..count).collect();
        let mut visited = 0;
        let found = graph.chains(&sources, &mut visited);
        let expected: Vec<Option<Vec<usize>>> = sources
            .iter()
            .map(|&source| reference_chain(source, &targets, &stops, &edges))
            .collect();
        prop_assert_eq!(found, expected);
        prop_assert!(visited <= count as u64, "the walk visits each node at most once");
    }
}
