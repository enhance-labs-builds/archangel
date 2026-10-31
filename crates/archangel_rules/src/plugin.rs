//! What the plugin rules see of a file: its element, captures and selectors, and the plugin rules
//! whose `files` it is in, all read from its class, so no file is classified for a plugin.

use std::collections::BTreeMap;

use archangel_config::{Classification, Condition, PluginUse};
use archangel_graph::{SourceFile, Work};

use crate::architecture::{Architecture, Origin};
use crate::classes::{ClassKey, Classes};
use crate::diagnostic::Diagnostic;

/// A file as the plugin rules see it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginPlace {
    /// The file's element.
    pub element: Option<String>,
    /// The element's captures, by name.
    pub captures: BTreeMap<String, String>,
    /// The declared selectors the file is in, in declaration order.
    pub selectors: Vec<String>,
    /// The plugin rules whose `files` hold the file, by their index in the config.
    pub rules: Vec<usize>,
}

/// What a plugin rule found in an example's files.
#[derive(Debug, Clone, Default)]
pub struct PluginFindings {
    /// Its violations, and its failures.
    pub diagnostics: Vec<Diagnostic>,
    /// Whether one of the example's files was in the rule's view, which is what an accepted
    /// example of a plugin rule needs to prove something.
    pub considered: bool,
}

impl Architecture {
    /// The rules that use a plugin condition, by their index in the config.
    pub fn plugin_rules(&self) -> impl Iterator<Item = (usize, &PluginUse)> + '_ {
        self.config
            .rules
            .iter()
            .enumerate()
            .filter_map(|(index, rule)| rule.plugin_use().map(|used| (index, used)))
    }

    /// A repository file as the plugin rules see it, from the class the session keeps.
    #[must_use]
    pub fn plugin_place(&self, classes: &Classes, path: &str) -> PluginPlace {
        match classes.bits(path) {
            Some(bits) => self.place_from(bits, classes.classification(path)),
            None => PluginPlace::default(),
        }
    }

    /// An example's file as the plugin rules see it, classified from its own text.
    #[must_use]
    pub fn example_plugin_place(&self, file: &SourceFile, work: &mut Work) -> PluginPlace {
        let mut classes = Classes::new();
        let (class, classification) = classes.example_file(self, file, work);
        match classes.key(class) {
            ClassKey::File { bits, .. } => self.place_from(bits, classification.as_ref()),
            ClassKey::Package { .. } => PluginPlace::default(),
        }
    }

    fn place_from(&self, bits: &[bool], classification: Option<&Classification>) -> PluginPlace {
        let set = |index: usize| bits.get(index) == Some(&true);
        let rules = self
            .sentences
            .iter()
            .filter(|compiled| {
                let sentence = &self.config.rules[compiled.rule].sentences[compiled.sentence];
                matches!(sentence.condition, Condition::Plugin(_)) && set(compiled.files)
            })
            .map(|compiled| compiled.rule)
            .collect();
        let selectors = self
            .reported
            .iter()
            .filter_map(|(index, origin)| match origin {
                Origin::Selector(name) if set(*index) => Some(name.clone()),
                _ => None,
            })
            .collect();
        PluginPlace {
            element: classification
                .map(|class| self.config.elements[class.element.index()].name.clone()),
            captures: classification
                .map(|class| {
                    class
                        .names
                        .iter()
                        .cloned()
                        .zip(class.captures.iter().cloned())
                        .collect()
                })
                .unwrap_or_default(),
            selectors,
            rules,
        }
    }
}
