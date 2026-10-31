//! The laws of patterns: the compiled matcher agrees with a slow matcher that follows the
//! definition in the glossary directly, by backtracking over folders and characters.
#![cfg(test)]

use archangel_config::Pattern;
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum Part {
    Lit(char),
    Star,
    Capture,
}

#[derive(Debug, Clone)]
enum Seg {
    Globstar,
    Parts(Vec<Part>),
}

fn render(segs: &[Seg]) -> String {
    let mut capture = 0;
    segs.iter()
        .map(|seg| match seg {
            Seg::Globstar => "**".to_owned(),
            Seg::Parts(parts) => parts
                .iter()
                .map(|part| match part {
                    Part::Lit(c) => c.to_string(),
                    Part::Star => "*".to_owned(),
                    Part::Capture => {
                        capture += 1;
                        format!("{{c{capture}}}")
                    }
                })
                .collect(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn reference(segs: &[Seg], path: &[&str]) -> bool {
    match segs.split_first() {
        None => path.is_empty(),
        Some((Seg::Globstar, [])) => !path.is_empty(),
        Some((Seg::Globstar, rest)) => (0..=path.len()).any(|skip| reference(rest, &path[skip..])),
        Some((Seg::Parts(parts), rest)) => match path.split_first() {
            Some((first, tail)) => name_matches(parts, first) && reference(rest, tail),
            None => false,
        },
    }
}

fn name_matches(parts: &[Part], name: &str) -> bool {
    match parts.split_first() {
        None => name.is_empty(),
        Some((Part::Lit(c), rest)) => name
            .strip_prefix(*c)
            .is_some_and(|tail| name_matches(rest, tail)),
        Some((Part::Star, rest)) => (0..=name.len()).any(|i| name_matches(rest, &name[i..])),
        Some((Part::Capture, rest)) => (1..=name.len()).any(|i| name_matches(rest, &name[i..])),
    }
}

fn part() -> impl Strategy<Value = Part> {
    prop_oneof![
        4 => prop::sample::select(vec!['a', 'b', '.']).prop_map(Part::Lit),
        1 => Just(Part::Star),
        1 => Just(Part::Capture),
    ]
}

fn seg() -> impl Strategy<Value = Seg> {
    prop_oneof![
        1 => Just(Seg::Globstar),
        // Two stars side by side would render as `**`, which is a different token.
        4 => prop::collection::vec(part(), 1..4).prop_map(|mut parts| {
            parts.dedup_by(|a, b| matches!((a, b), (Part::Star, Part::Star)));
            Seg::Parts(parts)
        }),
    ]
}

fn path() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(prop::string::string_regex("[ab.]{1,3}").unwrap(), 1..5)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn patterns_agree_with_the_reference_matcher(segs in prop::collection::vec(seg(), 1..5), path in path()) {
        let source = render(&segs);
        let pattern = Pattern::parse(&source).expect("generated patterns are valid");
        let parts: Vec<&str> = path.iter().map(String::as_str).collect();
        let expected = reference(&segs, &parts);
        prop_assert_eq!(pattern.matches(&path.join("/")).is_some(), expected, "pattern {} on {}", source, path.join("/"));
    }
}
