//! The rule table agrees with a slow reference that reads each condition's definition from the
//! spec directly, for random configs and random pairs of files; each shorthand agrees with the
//! sentences it stands for; and the transitive walk agrees with a search of every chain.
#![cfg(test)]

use archangel_graph::{Import, ImportForm, ImportKind};
use archangel_rules::{Architecture, ReachGraph, Target};
use proptest::prelude::*;
use serde_json::{Value, json};

const ELEMENTS: usize = 4;

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
        prop::collection::vec(0..=ELEMENTS, 1..3),
        imports(),
        instance,
        prop::option::of(0..=ELEMENTS),
    )
        .prop_map(|(files, condition, side, imports, instance, area)| {
            let mut rule = json!({
                "files": side_name(files),
                condition: side.into_iter().map(side_name).collect::<Vec<_>>(),
                "imports": imports,
            });
            if let (Some(area), true) = (area, condition != "mayNotImport") {
                rule["within"] = json!(side_name(area));
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
    kind: ImportKind,
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
    let (from_in, to_in) = (in_side(files, case.from), in_side(files, case.to));
    let instance_holds = match rule["instance"].as_str() {
        None => true,
        Some(wanted) => {
            let both = element_of(case.from).is_some_and(|f| name(f) == files)
                && element_of(case.to).is_some_and(|t| name(t) == files);
            both && (wanted == "same") == same_instance
        }
    };
    // Its own files are the files of the same instance when `files` is an element with a capture.
    let captured = files == name(0) || files == name(1);
    let own = to_in && (!captured || same_instance);
    let in_area = |end| {
        rule.get("within")
            .is_none_or(|area| in_side(area.as_str().unwrap(), end))
    };
    let condition = if rule.get("mayNotImport").is_some() {
        from_in && in_list(rule, "mayNotImport", case.to)
    } else if rule.get("mayOnlyImport").is_some() {
        from_in && in_area(case.to) && !in_list(rule, "mayOnlyImport", case.to) && !own
    } else {
        to_in && in_area(case.from) && !in_list(rule, "mayOnlyBeImportedBy", case.from) && !from_in
    };
    condition && instance_holds
}

fn import(kind: ImportKind) -> Import {
    Import {
        specifier: "./x".to_owned(),
        kind,
        form: ImportForm::Static,
        line: 1,
        column: 1,
        span: (0, 0),
        utf16: (0, 0),
        names: vec!["x".to_owned()],
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
        &json!({ "elements": elements(), "rules": named }).to_string(),
    )
    .expect("a valid config");
    Architecture::compile(config)
}

/// The rules an import breaks, in config order.
fn broken(architecture: &Architecture, case: &Case) -> Vec<usize> {
    let (from, to) = (path(case.from), path(case.to));
    let mut found = architecture.broken_rules(&from, Target::File(&to), &import(case.kind));
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn violations_agree_with_the_reference_evaluator(
        rules in prop::collection::vec(rule(), 1..6),
        from in end(),
        to in end(),
        typed in any::<bool>(),
    ) {
        let architecture = compile(&rules);
        let case = Case { from, to, kind: if typed { ImportKind::Type } else { ImportKind::Runtime } };
        let expected: Vec<usize> = rules.iter().enumerate()
            .filter(|(_, rule)| breaks(rule, &case))
            .map(|(index, _)| index)
            .collect();
        prop_assert_eq!(broken(&architecture, &case), expected);
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
        let case = Case { from, to, kind: ImportKind::Runtime };
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
        let case = Case { from, to, kind: if typed { ImportKind::Type } else { ImportKind::Runtime } };
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
