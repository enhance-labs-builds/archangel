//! One import against every rule, for `archangel explain <file>:<line>`: the verdict of each rule
//! that judged it, and why each other rule skipped it.

use archangel_config::{Classification, Condition, Rule, RuleKind, Sentence};
use archangel_graph::{Import, ImportGraph, Resolution, Work};

use crate::architecture::{Architecture, Compiled, Edge, End, covers, in_files};
use crate::check::Report;
use crate::classes::{ClassKey, Classes, Lookup};
use crate::exceptions;
use crate::messages::RowViolation;
use crate::words::DEFAULT_AREA;

/// What a rule did with one import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It judged the import and refuses it, and why.
    Refuses(String),
    /// It judged the import and permits it, and why.
    Permits(String),
    /// It did not judge the import, and why.
    Skips(String),
}

/// One rule's verdict on one import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleJudgement {
    /// The rule's name.
    pub rule: String,
    /// The verdict.
    pub verdict: Verdict,
    /// What permits a refused import all the same: an exception or the baseline.
    pub permitted_by: Option<String>,
}

/// One end of an import: its parts, each with its instance, and its kinds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportEnd {
    /// Each part, with the values of its captures.
    pub parts: Vec<(String, Vec<String>)>,
    /// Each table that gives the file a kind, with the kind.
    pub kinds: Vec<(String, String)>,
}

/// One import against every rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportJudgement {
    /// The importing file.
    pub from: ImportEnd,
    /// What it imports.
    pub to: ImportEnd,
    /// Every rule, in config order, and then the rules that `off` removes.
    pub rules: Vec<RuleJudgement>,
}

/// The import being judged, with its classes when it resolves.
struct Judged<'j> {
    architecture: &'j Architecture,
    classes: &'j Classes,
    edge: Option<Edge<'j>>,
    from: &'j ClassKey,
    to: Option<&'j ClassKey>,
    path: &'j str,
    report: &'j Report,
    import: &'j Import,
    exceptions: &'j [exceptions::Exception],
    target: &'j str,
}

impl Architecture {
    /// Judges one import of a file against every rule.
    #[must_use]
    pub fn judge_import(
        &self,
        graph: &ImportGraph,
        path: &str,
        import: &Import,
        resolution: &Resolution,
        report: &Report,
    ) -> ImportJudgement {
        let mut classes = Classes::new();
        let mut work = Work::default();
        let lookup = Lookup::new(self, graph);
        let (from_class, from) = classes.file(lookup, path, &mut work);
        let (to_class, to, to_path, package) = self.target(&mut classes, lookup, resolution);
        if let Some(to_class) = to_class {
            classes.fill_cell(self, from_class, to_class);
        }
        let edge = to_class.map(|to_class| {
            let from = End {
                class: from_class,
                classification: from.as_ref(),
                path: Some(path),
                package: None,
            };
            let to = End {
                class: to_class,
                classification: to.as_ref(),
                path: to_path,
                package,
            };
            Edge {
                from,
                to,
                file: path,
                import,
                resolution,
                manifests: Some(graph.manifests_with(None)),
            }
        });
        let found = exceptions_of(graph, path);
        let judged = Judged {
            architecture: self,
            classes: &classes,
            edge,
            from: classes.key(from_class),
            to: to_class.map(|class| classes.key(class)),
            path,
            report,
            import,
            exceptions: &found,
            target: to_path.or(package).unwrap_or_default(),
        };
        let rules = self.rule_judgements(&judged);
        ImportJudgement {
            from: self.end_of(from.as_ref(), Some(path), &classes),
            to: self.end_of(to.as_ref(), to_path, &classes),
            rules,
        }
    }

    /// The class, the place and the name of what an import resolves to.
    fn target<'r>(
        &self,
        classes: &mut Classes,
        lookup: Lookup<'_>,
        resolution: &'r Resolution,
    ) -> (
        Option<u32>,
        Option<Classification>,
        Option<&'r str>,
        Option<&'r str>,
    ) {
        match resolution {
            Resolution::File(to) => {
                let (class, found) = classes.file(lookup, to, &mut Work::default());
                (Some(class), found, Some(to.as_str()), None)
            }
            Resolution::Package(name) => {
                let (class, found) = classes.package(self, name);
                (Some(class), found, None, Some(name.as_str()))
            }
            Resolution::Unresolved(_) => (None, None, None, None),
        }
    }

    /// The verdict of each rule on one import, and of each rule that `off` removes.
    fn rule_judgements(&self, judged: &Judged<'_>) -> Vec<RuleJudgement> {
        let mut rules: Vec<RuleJudgement> = self
            .config
            .rules
            .iter()
            .enumerate()
            .map(|(index, rule)| judged.rule(index, rule))
            .collect();
        rules.extend(self.config.off.iter().map(|rule| RuleJudgement {
            rule: rule.name.clone(),
            verdict: Verdict::Skips("the rule is off".to_owned()),
            permitted_by: None,
        }));
        rules
    }

    fn end_of(
        &self,
        class: Option<&Classification>,
        path: Option<&str>,
        classes: &Classes,
    ) -> ImportEnd {
        ImportEnd {
            parts: class.map_or_else(Vec::new, |class| {
                class
                    .all()
                    .map(|place| {
                        (
                            self.config.elements[place.element.index()].name.clone(),
                            place.captures.clone(),
                        )
                    })
                    .collect()
            }),
            kinds: path.map_or_else(Vec::new, |path| self.kinds_in(classes, path)),
        }
    }
}

impl Judged<'_> {
    /// The strongest verdict of a rule's sentences: a refusal, then a permission, then a skip.
    fn rule(&self, index: usize, rule: &Rule) -> RuleJudgement {
        let verdicts: Vec<Verdict> = self
            .architecture
            .sentences
            .iter()
            .enumerate()
            .filter(|(_, compiled)| compiled.rule == index)
            .map(|(place, compiled)| self.sentence(place, compiled))
            .collect();
        let chosen = |wanted: fn(&Verdict) -> bool| verdicts.iter().find(|v| wanted(v)).cloned();
        let verdict = chosen(|v| matches!(v, Verdict::Refuses(_)))
            .or_else(|| chosen(|v| matches!(v, Verdict::Permits(_))))
            .or_else(|| verdicts.first().cloned())
            .unwrap_or_else(|| Verdict::Skips("the rule has no sentence".to_owned()));
        let permitted_by = matches!(verdict, Verdict::Refuses(_))
            .then(|| self.permitted_by(&rule.name))
            .flatten();
        RuleJudgement {
            rule: rule.name.clone(),
            verdict,
            permitted_by,
        }
    }

    fn sentence(&self, place: usize, compiled: &Compiled) -> Verdict {
        let rule = &self.architecture.config.rules[compiled.rule];
        let sentence = &rule.sentences[compiled.sentence];
        let about_target = match condition_scope(rule, sentence) {
            Ok(about_target) => about_target,
            Err(skipped) => return skipped,
        };
        if !about_target && !in_files(compiled, self.from) {
            return self.not_in_files(rule, sentence);
        }
        let (Some(edge), Some(to)) = (self.edge.as_ref(), self.to) else {
            return skip("the import does not resolve");
        };
        if about_target && !in_files(compiled, to) {
            return skip(&format!("the target is not in {}", sentence.files_text));
        }
        if !covers(rule.imports, self.import.kind) {
            return skip("the rule does not count this kind of import");
        }
        let other_end = if about_target { self.from } else { to };
        if let (Some(area), Some(written)) = (&compiled.area, &sentence.options.within)
            && !area.holds(other_end)
        {
            return skip(&self.outside_area(about_target, &written.text));
        }
        self.decided((place, compiled), edge, other_end)
    }

    /// The verdict on an import that the sentence looks at.
    fn decided(
        &self,
        (place, compiled): (usize, &Compiled),
        edge: &Edge<'_>,
        other_end: &ClassKey,
    ) -> Verdict {
        let sentence = &self.architecture.config.rules[compiled.rule].sentences[compiled.sentence];
        if let Some(check) = self
            .classes
            .cell(edge.from.class, edge.to.class)
            .iter()
            .find(|check| check.sentence == place)
            && self.architecture.test(check, edge, self.classes)
        {
            return Verdict::Refuses(self.refusal(compiled, edge));
        }
        if !crate::facts::hold(sentence, edge, self.classes)
            && let Some(limit) = limit_text(sentence)
        {
            return skip(&limit);
        }
        Verdict::Permits(permission(self.architecture, compiled, other_end))
    }

    fn not_in_files(&self, rule: &Rule, sentence: &Sentence) -> Verdict {
        let files = &sentence.files_text;
        let config = &self.architecture.config;
        let RuleKind::TableRow {
            table,
            row: Some(row),
        } = &rule.kind
        else {
            return skip(&format!("the file is not in {files}"));
        };
        let is_kind = config.tables[table.index()].kinds.is_some()
            && config.element(row).is_none()
            && config.selector(row).is_none();
        if !is_kind {
            return skip(&format!("the file is not in {files}"));
        }
        match self.classes.kind_place(self.path, *table) {
            Some(place) if place.kind == *row => skip("the file is outside the table's files"),
            Some(place) => skip(&format!("the file has the kind {}, not {row}", place.kind)),
            None => skip(&format!(
                "the file has no kind in {}",
                config.tables[table.index()].name
            )),
        }
    }

    fn refusal(&self, compiled: &Compiled, edge: &Edge<'_>) -> String {
        let architecture = self.architecture;
        let text = architecture.sentence_text(compiled.rule, compiled.sentence);
        let rule = &architecture.config.rules[compiled.rule];
        let sentence = &rule.sentences[compiled.sentence];
        let RuleKind::TableRow {
            table,
            row: Some(row),
        } = &rule.kind
        else {
            return text;
        };
        if matches!(sentence.condition, Condition::MayNotImport(_)) {
            return text;
        }
        let name = RowViolation {
            architecture,
            classes: self.classes,
            table: *table,
            row,
            compiled,
            to: edge.to.classification,
            path: edge.to.path,
            package: edge.to.package,
        }
        .target_name();
        format!("{name} is not in its row ({text})")
    }

    /// Why an import is outside a rule's area, in the terms of the end that the area holds.
    fn outside_area(&self, about_target: bool, area: &str) -> String {
        let end = if about_target { self.path } else { self.target };
        if area == DEFAULT_AREA {
            format!("{end} is outside {area}")
        } else {
            format!("{end} is outside the area {area}")
        }
    }

    /// What permits a violation of a rule on this line: an exception, or else the baseline when
    /// the check does not report it.
    fn permitted_by(&self, rule: &str) -> Option<String> {
        if let Some(exception) = self
            .exceptions
            .iter()
            .find(|e| e.rule == rule && e.target_line == self.import.line)
        {
            return Some(format!("an exception: {}", exception.reason));
        }
        let reported = self.report.diagnostics.iter().any(|d| {
            d.rule.as_deref() == Some(rule)
                && d.file.as_deref() == Some(self.path)
                && d.line == self.import.line
        });
        (!reported).then(|| "the baseline".to_owned())
    }
}

fn skip(reason: &str) -> Verdict {
    Verdict::Skips(reason.to_owned())
}

/// Whether a sentence is about the target of an import, or the skip of a sentence that judges no
/// single import.
fn condition_scope(rule: &Rule, sentence: &Sentence) -> Result<bool, Verdict> {
    if sentence.options.transitively {
        return Err(skip(
            "a chain rule checks chains of imports, not one import",
        ));
    }
    match &sentence.condition {
        Condition::MayNotImport(_) | Condition::MayOnlyImport(_) => Ok(false),
        Condition::MayOnlyBeImportedBy(_) | Condition::Entry { .. } => Ok(true),
        Condition::NoCycles => Err(skip("noCycles checks cycles, not one import")),
        Condition::MustNotExist if matches!(rule.kind, RuleKind::TableRow { .. }) => {
            Err(skip("this rule checks files, not imports"))
        }
        other => Err(skip(&format!("{} checks no import", other.name()))),
    }
}

/// Which option of a sentence makes it pass an import by, when one is set.
fn limit_text(sentence: &Sentence) -> Option<String> {
    let options = &sentence.options;
    if options.names.is_some() {
        return Some("the import does not bring in a name that the rule names".to_owned());
    }
    if options.specifier.is_some() {
        return Some("the specifier is not written the way that the rule names".to_owned());
    }
    if options.dependency.is_some() {
        return Some(
            "the manifest does not list the package the way that the rule names".to_owned(),
        );
    }
    options
        .instance
        .map(|_| "the instance differs from what the rule names".to_owned())
}

/// Why a sentence that judged an import permits it.
fn permission(architecture: &Architecture, compiled: &Compiled, other_end: &ClassKey) -> String {
    let rule = &architecture.config.rules[compiled.rule];
    let sentence = &rule.sentences[compiled.sentence];
    let side = sentence
        .condition
        .side()
        .map_or("", |side| side.text.as_str());
    match &sentence.condition {
        Condition::MayNotImport(_) => format!("the target is not in {side}"),
        Condition::MayOnlyImport(_) if compiled.side.holds(other_end) => {
            if matches!(rule.kind, RuleKind::TableRow { .. }) {
                "the target is in its row".to_owned()
            } else {
                format!("the target is in {side}")
            }
        }
        Condition::MayOnlyImport(_) if compiled.own_instance.is_some() => {
            format!("the same instance of {}", sentence.files_text)
        }
        Condition::MayOnlyImport(_) => format!("the target is in {}", sentence.files_text),
        Condition::MayOnlyBeImportedBy(_) if compiled.side.holds(other_end) => {
            format!("the file is in {side}")
        }
        Condition::MayOnlyBeImportedBy(_) => format!("the file is in {}", sentence.files_text),
        _ => "the import stays in one instance, or reaches an entry file".to_owned(),
    }
}

/// The exception comments of a file.
fn exceptions_of(graph: &ImportGraph, path: &str) -> Vec<exceptions::Exception> {
    graph
        .file(path)
        .and_then(|file| file.facts.as_ref())
        .map(|facts| exceptions::read(&facts.allow_comments))
        .unwrap_or_default()
}
