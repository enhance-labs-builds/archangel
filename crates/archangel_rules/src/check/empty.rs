//! The parts of a check that find what matches nothing: a group, a part, a preset or a package name
//! that a rule names, each of which would make a rule that cannot fire.

use std::collections::BTreeSet;

use archangel_config::{Condition, ElementId, ElementKind, Pattern, RuleKind};

use super::Run;
use crate::architecture::Origin;
use crate::diagnostic::{Code, Diagnostic};
use crate::messages::pattern_list;

impl Run<'_> {
    /// A declared selector, or an inline pattern of a rule, that matches no file the walk visited.
    /// What a preset declared is listed in the summary instead.
    pub(super) fn empty_selectors(&mut self) {
        for (set, origin) in &self.architecture.reported {
            if self.classes.files_of_set(*set) > 0 {
                continue;
            }
            let name = match origin {
                Origin::Selector(name) | Origin::Inline(name) => name,
            };
            if self.architecture.from_preset(name) {
                if let Origin::Selector(name) = origin {
                    self.summary
                        .preset_names_matching_nothing
                        .push(name.clone());
                }
                continue;
            }
            let definition = self.architecture.definition(*set);
            let message = match origin {
                Origin::Selector(name) => format!(
                    "the {} \"{name}\" matches no file ({definition}), so no rule about it can \
                     fire; fix its definition or remove it",
                    self.architecture.config.group_word()
                ),
                Origin::Inline(rule) => format!(
                    "the rule \"{rule}\" names the pattern {definition}, which matches no file, so \
                     the rule cannot fire there; fix the pattern"
                ),
            };
            let mut diagnostic = Diagnostic::new(Code::SelectorEmpty, message);
            if let Origin::Inline(rule) = origin {
                diagnostic.rule = Some(rule.clone());
            }
            self.other.push(diagnostic);
        }
    }

    pub(super) fn empty_elements(&mut self) {
        let dependencies = self.graph.manifest_dependencies();
        // A file in two parts is in one of them only, but both patterns match it: `part-overlap`
        // reports it, and neither part matches nothing.
        let overlapping: BTreeSet<ElementId> = self
            .classes
            .overlaps()
            .values()
            .flatten()
            .copied()
            .collect();
        for (index, element) in self.architecture.config.elements.iter().enumerate() {
            let id = ElementId(u32::try_from(index).unwrap_or(u32::MAX));
            if self.used_elements.contains(&id) {
                continue;
            }
            let (listed, what) = match &element.kind {
                ElementKind::Files(patterns) => (
                    self.classes.files_of(id) > 0 || overlapping.contains(&id),
                    pattern_list(patterns.iter().map(Pattern::source)),
                ),
                ElementKind::Packages(patterns) => (
                    // Node's built-in modules are always installed, though no manifest lists them.
                    patterns.iter().any(|p| p.source().starts_with("node:"))
                        || dependencies.iter().any(|name| {
                            self.architecture
                                .matcher
                                .package_elements(name)
                                .contains(&id)
                        }),
                    pattern_list(patterns.iter().map(Pattern::source)),
                ),
            };
            if !listed && self.architecture.from_preset(&element.name) {
                self.summary
                    .preset_names_matching_nothing
                    .push(element.name.clone());
            } else if !listed {
                let word = self.architecture.config.part_word();
                let message = format!(
                    "the {word} \"{}\" matches nothing ({what}), so no rule about it can fire; fix the \
                     pattern or remove the {word}",
                    element.name
                );
                self.other
                    .push(Diagnostic::new(Code::ElementEmpty, message));
            }
        }
        self.empty_package_names();
        self.empty_presets();
    }

    /// The sets of the rows of each table that a preset declares.
    pub(super) fn tables_of_preset(&self, preset: &str) -> Vec<usize> {
        let architecture = self.architecture;
        let prefix = format!("{preset}/");
        architecture
            .sentences
            .iter()
            .filter(|compiled| compiled.sentence == 0)
            .filter_map(
                |compiled| match &architecture.config.rules[compiled.rule].kind {
                    RuleKind::TableRow {
                        table,
                        row: Some(_),
                    } if architecture.config.tables[table.index()]
                        .name
                        .starts_with(&prefix) =>
                    {
                        Some(compiled.files)
                    }
                    _ => None,
                },
            )
            .collect()
    }

    /// A preset whose file elements and tables together match no file: most likely a wrong root.
    pub(super) fn empty_presets(&mut self) {
        for preset in &self.architecture.config.presets {
            let files: Vec<ElementId> = preset
                .elements
                .iter()
                .copied()
                .filter(|id| {
                    matches!(
                        self.architecture.config.elements[id.index()].kind,
                        ElementKind::Files(_)
                    )
                })
                .collect();
            let tables = self.tables_of_preset(&preset.name);
            let tables_have_files = tables
                .iter()
                .any(|set| self.classes.table_files_of_set(*set) > 0);
            if (files.is_empty() && tables.is_empty())
                || tables_have_files
                || files.iter().any(|&id| self.classes.files_of(id) > 0)
            {
                continue;
            }
            let root = if preset.root.is_empty() {
                "the config folder".to_owned()
            } else {
                format!("`{}`", preset.root)
            };
            let message = format!(
                "the preset \"{}\" matches no file under {root}, so none of its rules can fire; give \
                 it the folder it applies to as its root",
                preset.name
            );
            self.other.push(Diagnostic::new(Code::PresetEmpty, message));
        }
    }

    /// An inline package name on the side of a rule that permits imports, which no import reaches
    /// and no manifest lists: a misspelt package there means the rule refuses what it should
    /// permit. A `mayNotImport` side is exempt, because forbidding a package before anyone
    /// installs it is that rule's passing state.
    pub(super) fn empty_package_names(&mut self) {
        let dependencies = self.graph.manifest_dependencies();
        let mut reported: BTreeSet<(usize, usize)> = BTreeSet::new();
        for compiled in &self.architecture.sentences {
            let rule = &self.architecture.config.rules[compiled.rule];
            if !matches!(
                rule.sentences[compiled.sentence].condition,
                Condition::MayOnlyImport(_) | Condition::MayOnlyBeImportedBy(_)
            ) {
                continue;
            }
            let area = compiled.area.iter().flat_map(|area| &area.packages);
            for &name in compiled.side.packages.iter().chain(area) {
                let pattern = &self.architecture.package_names[name];
                let always = pattern.source() == "*" || pattern.source().starts_with("node:");
                let listed = always
                    || self.used_names.contains(&name)
                    || dependencies
                        .iter()
                        .any(|dependency| self.architecture.package_named(name, dependency));
                if listed || !reported.insert((compiled.rule, name)) {
                    continue;
                }
                let message = format!(
                    "the rule \"{}\" names the package `{}`, which no import uses and no package \
                     manifest lists, so the rule never permits it; fix the name",
                    rule.name,
                    pattern.source()
                );
                let mut diagnostic = Diagnostic::new(Code::ElementEmpty, message);
                diagnostic.rule = Some(rule.name.clone());
                self.other.push(diagnostic);
            }
        }
    }
}
