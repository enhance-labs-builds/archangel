//! Counts of the work a check did. Tests assert these instead of durations.

/// One counter per kind of work. Each stage adds to the counters of its own work; this struct
/// lives in the graph crate because it is the lowest crate that every counting stage uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Work {
    /// Source files that the walk found.
    pub files_found: u64,
    /// Source files read from disk.
    pub files_read: u64,
    /// Bytes read from those files.
    pub bytes_read: u64,
    /// Source files parsed.
    pub files_parsed: u64,
    /// Imports found in parsed files.
    pub imports_found: u64,
    /// Calls to the resolver.
    pub resolutions: u64,
    /// Files put into their element and selectors.
    pub files_classified: u64,
    /// Passes of the patterns over a path: one per file classified, or one per pattern when the
    /// pattern set was too large to compile as one.
    pub pattern_passes: u64,
    /// Checks of one import against one rule.
    pub rule_evaluations: u64,
    /// Files that cycle searches visited.
    pub cycle_files_visited: u64,
    /// Files that the walks of transitive rules visited.
    pub transitive_files_visited: u64,
    /// Looks of a naming or structure rule at one file or one instance.
    pub structure_evaluations: u64,
    /// Looks of a folder rule at one folder.
    pub folder_evaluations: u64,
    /// Looks of an export rule at one file.
    pub export_evaluations: u64,
    /// Rule examples checked.
    pub examples_checked: u64,
    /// Files written to disk.
    pub files_written: u64,
    /// File records sent to the plugin process, and records it was told to remove.
    pub plugin_records_sent: u64,
    /// Plugin conditions that the plugin process ran, one per plugin rule in each batch.
    pub plugin_checks: u64,
}

impl Work {
    /// Each counter with its name in output, in the order the spec lists them.
    #[must_use]
    pub fn counts(&self) -> [(&'static str, u64); 18] {
        [
            ("filesFound", self.files_found),
            ("filesRead", self.files_read),
            ("bytesRead", self.bytes_read),
            ("filesParsed", self.files_parsed),
            ("importsFound", self.imports_found),
            ("resolutions", self.resolutions),
            ("filesClassified", self.files_classified),
            ("patternPasses", self.pattern_passes),
            ("ruleEvaluations", self.rule_evaluations),
            ("cycleFilesVisited", self.cycle_files_visited),
            ("transitiveFilesVisited", self.transitive_files_visited),
            ("structureEvaluations", self.structure_evaluations),
            ("folderEvaluations", self.folder_evaluations),
            ("exportEvaluations", self.export_evaluations),
            ("examplesChecked", self.examples_checked),
            ("filesWritten", self.files_written),
            ("pluginRecordsSent", self.plugin_records_sent),
            ("pluginChecks", self.plugin_checks),
        ]
    }

    /// The counts as a JSON object, keyed by their names in output.
    #[must_use]
    pub fn counts_json(&self) -> serde_json::Value {
        serde_json::Value::Object(
            self.counts()
                .into_iter()
                .map(|(name, count)| (name.to_owned(), count.into()))
                .collect(),
        )
    }

    /// Adds another count to this one, after a parallel stage.
    pub fn add(&mut self, other: &Self) {
        self.files_found += other.files_found;
        self.files_read += other.files_read;
        self.bytes_read += other.bytes_read;
        self.files_parsed += other.files_parsed;
        self.imports_found += other.imports_found;
        self.resolutions += other.resolutions;
        self.files_classified += other.files_classified;
        self.pattern_passes += other.pattern_passes;
        self.rule_evaluations += other.rule_evaluations;
        self.cycle_files_visited += other.cycle_files_visited;
        self.transitive_files_visited += other.transitive_files_visited;
        self.structure_evaluations += other.structure_evaluations;
        self.folder_evaluations += other.folder_evaluations;
        self.export_evaluations += other.export_evaluations;
        self.examples_checked += other.examples_checked;
        self.files_written += other.files_written;
        self.plugin_records_sent += other.plugin_records_sent;
        self.plugin_checks += other.plugin_checks;
    }
}
