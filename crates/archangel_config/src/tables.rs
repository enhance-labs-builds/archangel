//! Reading an import table: each row compiles to one rule of ordinary sentences, and every table
//! makes the rule `<table>.every-file-has-a-kind`.

use serde_json::{Map, Value};

use crate::merge::ConfigPart;
use crate::model::{
    Condition, ElementId, ElementKind, Examples, FileSet, ImportKinds, ImportOptions, KindPattern,
    OwnFiles, OwnInstance, Rule, RuleKind, RuleSource, Sentence, Side, Table, TableId, Term,
};
use crate::parse::{
    nearest, one_spelling, optional_string, parse_examples, rooted, string_list, unknown_fields,
};
use crate::pattern::{KIND, Pattern};
use crate::problem::{ConfigProblem, ProblemCode};
use crate::rules::Input;
use crate::selectors::Names;

const FIELDS: [&str; 17] = [
    "kind",
    "name",
    "rows",
    "files",
    "kinds",
    "within",
    "everyRowMayImport",
    "mayNotImportItself",
    "because",
    "reason",
    "link",
    "imports",
    "names",
    "specifier",
    "dependency",
    "instance",
    "examples",
];

/// The name of the rule that every table makes, after the table's name and a dot.
pub const EVERY_FILE_HAS_A_KIND: &str = "every-file-has-a-kind";

/// One row, before its cells are read.
struct Row<'v> {
    key: &'v str,
    cells: &'v Value,
    files: FileSet,
    /// What makes an instance of the row's name, when something does.
    instance: Option<OwnInstance>,
}

/// Where a table is read: the part of the config it is in, and what reading it adds to.
pub(crate) struct TableReader<'r, 'v, 'n> {
    pub(crate) source: &'r ConfigPart<'v>,
    pub(crate) names: &'r mut Names<'n>,
    pub(crate) tables: &'r mut Vec<Table>,
    pub(crate) problems: &'r mut Vec<ConfigProblem>,
}

/// One table being read.
struct Reading<'r, 'v, 'n> {
    input: Input<'v>,
    id: TableId,
    full_name: String,
    kinds: Option<KindPattern>,
    names: &'r mut Names<'n>,
    problems: &'r mut Vec<ConfigProblem>,
}

/// What a table's rules share.
struct Shared {
    source: Option<RuleSource>,
    full_name: String,
    table: TableId,
    files: FileSet,
    default_files: bool,
    options: ImportOptions,
    default_area: Side,
    reason: Option<String>,
    because: bool,
    link: Option<String>,
    imports: ImportKinds,
}

impl TableReader<'_, '_, '_> {
    /// Reads one table, adds it to the tables, and returns its rules.
    pub(crate) fn read(&mut self, value: &Value, field: &str) -> Vec<Rule> {
        let Some(object) = value.as_object() else {
            return Vec::new();
        };
        let input = Input {
            object,
            field: field.to_owned(),
        };
        let before = self.problems.len();
        unknown_fields(object, &FIELDS, field, self.problems);
        let Some(written_name) = table_name(&input, self.problems) else {
            return Vec::new();
        };
        let full_name = self.names.scope.name(written_name);
        if self.tables.iter().any(|table| table.name == full_name) {
            self.problems.push(
                ConfigProblem::new(
                    ProblemCode::RuleNameDuplicate,
                    format!(
                        "two tables are named `{full_name}`; give each table its own name, because \
                         exceptions and the baseline refer to its rows by name"
                    ),
                )
                .at(input.at("name")),
            );
            return Vec::new();
        }
        let id = TableId(u32::try_from(self.tables.len()).unwrap_or(u32::MAX));
        let mut reading = Reading {
            input,
            id,
            full_name,
            kinds: None,
            names: self.names,
            problems: self.problems,
        };
        let Some((table, rules)) = reading.table(self.source, before) else {
            return Vec::new();
        };
        self.tables.push(table);
        rules
    }
}

impl<'v> Reading<'_, 'v, '_> {
    fn object(&self) -> &'v Map<String, Value> {
        self.input.object
    }

    /// The table and its rules, unless reading it found a problem since `before`.
    fn table(&mut self, source: &ConfigPart<'_>, before: usize) -> Option<(Table, Vec<Rule>)> {
        self.kinds = self.kind_pattern();
        if self.object().contains_key("kinds") && self.kinds.is_none() {
            return None;
        }
        let rows = self.rows()?;
        let (files, default_files) = self.table_files(&rows);
        let options = self.input.read_options(self.names, self.problems);
        let keys: Vec<&str> = rows.iter().map(|row| row.key).collect();
        let refused = self.refused_itself(&keys);
        let every = self.every_row_cells(&rows);
        let sides: Vec<Side> = rows
            .iter()
            .map(|row| self.row_side(row, &rows, &refused))
            .collect();
        let shared = self.shared(files, default_files, options, default_area(&sides, &every));
        let mut rules = Vec::with_capacity(rows.len() + 1);
        for (row, side) in rows.iter().zip(sides) {
            self.needs_a_capture(row, &shared);
            let refuses_itself = refused.iter().any(|name| name == row.key);
            rules.push(shared.row_rule(row, with_every_row(side, &every), refuses_itself));
        }
        rules.push(shared.every_file_rule(&rows));
        let table = self.table_of(shared, &rows, source);
        (self.problems.len() == before).then_some((table, rules))
    }

    fn table_of(&mut self, shared: Shared, rows: &[Row<'_>], source: &ConfigPart<'_>) -> Table {
        let examples = parse_examples(
            self.object().get("examples"),
            &self.input.field,
            self.problems,
        );
        let from_preset = source.scope.prefix.is_some();
        Table {
            name: shared.full_name,
            kinds: self.kinds.take(),
            files: shared.files,
            default_files: shared.default_files,
            rows: rows
                .iter()
                .map(|row| (row.key.to_owned(), row.files.clone()))
                .collect(),
            examples: if from_preset {
                rooted(examples, &source.scope)
            } else {
                examples
            },
            from_preset,
        }
    }

    fn shared(
        &mut self,
        files: FileSet,
        default_files: bool,
        options: ImportOptions,
        default_area: Side,
    ) -> Shared {
        let object = self.object();
        let reason_key = one_spelling(
            object,
            ["because", "reason"],
            |key| self.input.at(key),
            self.problems,
        )
        .map_or("reason", |(key, _)| key);
        Shared {
            source: crate::parse::rule_source(object),
            default_area,
            full_name: self.full_name.clone(),
            table: self.id,
            files,
            default_files,
            options,
            reason: optional_string(object, reason_key, &self.input.field, self.problems),
            because: reason_key == "because",
            link: optional_string(object, "link", &self.input.field, self.problems),
            imports: self.input.imports(&[], false, self.problems),
        }
    }

    fn needs_a_capture(&mut self, row: &Row<'_>, shared: &Shared) {
        if shared.options.instance.is_some() && row.instance.is_none() {
            self.problems.push(
                ConfigProblem::new(
                    ProblemCode::RuleNeedsCapture,
                    format!(
                        "`instance` compares the instances at both ends, and the row `{}` has no \
                         capture, so it has one instance",
                        row.key
                    ),
                )
                .at(self.input.at(&format!("rows.{}", row.key))),
            );
        }
    }

    fn kind_pattern(&mut self) -> Option<KindPattern> {
        let value = self.object().get("kinds")?;
        let field = self.input.at("kinds");
        let Value::String(text) = value else {
            self.problems.push(invalid(
                &field,
                "must be a pattern with the capture `{kind}`",
            ));
            return None;
        };
        let pattern = match Pattern::parse_kinds(&self.names.scope.path(text)) {
            Ok(pattern) => pattern,
            Err(error) => {
                self.problems.push(
                    ConfigProblem::new(
                        ProblemCode::PatternInvalid,
                        format!(
                            "the kind pattern `{text}` of the table \"{}\" is invalid: {error}",
                            self.full_name
                        ),
                    )
                    .at(field),
                );
                return None;
            }
        };
        if !pattern.captures().iter().any(|name| name == KIND) {
            self.problems.push(invalid(
                &field,
                &format!("the kind pattern `{text}` must contain the capture `{{kind}}`"),
            ));
            return None;
        }
        let id = self.names.intern(pattern.clone());
        Some(KindPattern { pattern, id })
    }

    fn rows(&mut self) -> Option<Vec<Row<'v>>> {
        let Some(written) = self.object().get("rows").and_then(Value::as_object) else {
            self.problems.push(invalid(
                &self.input.at("rows"),
                "must map each row to the list of what it may import",
            ));
            return None;
        };
        Some(
            written
                .iter()
                .filter_map(|(key, cells)| self.row(key, cells))
                .collect(),
        )
    }

    /// A row key means a declared part or group, and otherwise, with a kind pattern, a kind.
    fn row(&mut self, key: &'v str, cells: &'v Value) -> Option<Row<'v>> {
        let field = self.input.at(&format!("rows.{key}"));
        if !refuse_character(key, &field, self.problems) {
            return None;
        }
        let scoped = self.names.scope.name(key);
        if let Some(index) = self.names.elements.iter().position(|e| e.name == scoped) {
            let element = &self.names.elements[index];
            if matches!(element.kind, ElementKind::Packages(_)) {
                self.problems.push(invalid(
                    &field,
                    &format!(
                        "\"{key}\" is a set of outside packages, and outside packages have no files \
                         that import, so they cannot be a row; name it in a cell"
                    ),
                ));
                return None;
            }
            let id = ElementId(u32::try_from(index).unwrap_or(u32::MAX));
            return Some(Row {
                key,
                cells,
                files: FileSet::of(Term::Element(id)),
                instance: element.has_capture().then_some(OwnInstance::Element(id)),
            });
        }
        if let Some(selector) = self.names.selector_id(&scoped) {
            return Some(Row {
                key,
                cells,
                files: FileSet::of(Term::Selector(selector)),
                instance: None,
            });
        }
        let Some(kinds) = &self.kinds else {
            self.problems
                .push(unknown_name(key, &[], &field, self.names));
            return None;
        };
        Some(Row {
            key,
            cells,
            files: FileSet::of(Term::Kind {
                table: self.id,
                kind: key.to_owned(),
            }),
            instance: kinds.has_instance().then_some(OwnInstance::Kind(self.id)),
        })
    }

    /// The files the table judges, and whether they are its default files.
    fn table_files(&mut self, rows: &[Row<'_>]) -> (FileSet, bool) {
        if let Some(files) = self.object().get("files") {
            let set = self
                .names
                .file_set(files, &self.input.at("files"), self.problems)
                .unwrap_or_default();
            return (set, false);
        }
        let mut sets: Vec<FileSet> = Vec::new();
        if self.kinds.is_some() {
            sets.push(FileSet::of(Term::AnyKind(self.id)));
        }
        sets.extend(
            rows.iter()
                .filter(|row| !matches!(row.files.terms.as_slice(), [Term::Kind { .. }]))
                .map(|row| row.files.clone()),
        );
        (FileSet::any(&sets), true)
    }

    /// The rows that `mayNotImportItself` lists, each a row of the table.
    fn refused_itself(&mut self, keys: &[&str]) -> Vec<String> {
        let field = self.input.at("mayNotImportItself");
        let listed = string_list(
            self.object().get("mayNotImportItself"),
            &field,
            self.problems,
        );
        for name in &listed {
            if !keys.contains(&name.as_str()) {
                self.problems.push(
                    ConfigProblem::new(
                        ProblemCode::RuleUnknownElement,
                        format!(
                            "`mayNotImportItself` lists \"{name}\", and the table has no such row; \
                             {}the rows are {}",
                            nearest(name, keys)
                                .map(|near| format!("did you mean \"{near}\"? "))
                                .unwrap_or_default(),
                            keys.join(", ")
                        ),
                    )
                    .at(&field),
                );
            }
        }
        listed
    }

    /// The side of a list of cells.
    fn cells(&mut self, cells: &[Value], rows: &[Row<'_>], field: &str) -> Side {
        let mut side = Side::default();
        for (index, cell) in cells.iter().enumerate() {
            if let Some(found) = self.cell(cell, rows, &format!("{field}[{index}]")) {
                side.files.extend(found.files);
                side.elements.extend(found.elements);
                side.packages.extend(found.packages);
            }
            if let Some(text) = cell.as_str() {
                join_text(&mut side.text, text);
            }
        }
        side
    }

    /// What one cell names.
    fn cell(&mut self, cell: &Value, rows: &[Row<'_>], field: &str) -> Option<Side> {
        let Value::String(text) = cell else {
            self.problems.push(invalid(
                field,
                "must name a row, a part, a group, an externals set or a pattern",
            ));
            return None;
        };
        if let Some(row) = rows.iter().find(|row| row.key == text) {
            return Some(Side {
                files: vec![row.files.clone()],
                ..Side::default()
            });
        }
        if !self.names.declares(text) && !text.contains('/') && !text.contains('*') {
            let keys: Vec<&str> = rows.iter().map(|row| row.key).collect();
            self.problems
                .push(unknown_name(text, &keys, field, self.names));
            return None;
        }
        self.names.side(cell, field, self.problems)
    }

    /// The cells that every row also has.
    fn every_row_cells(&mut self, rows: &[Row<'_>]) -> Side {
        let Some(value) = self.object().get("everyRowMayImport") else {
            return Side::default();
        };
        let field = self.input.at("everyRowMayImport");
        let Some(cells) = value.as_array() else {
            self.problems
                .push(invalid(&field, "must be a list of cells"));
            return Side::default();
        };
        self.cells(cells, rows, &field)
    }

    /// The cells of one row, without those of `everyRowMayImport`.
    fn row_side(&mut self, row: &Row<'_>, rows: &[Row<'_>], refused: &[String]) -> Side {
        let field = self.input.at(&format!("rows.{}", row.key));
        let every_row = self
            .object()
            .get("everyRowMayImport")
            .and_then(Value::as_array);
        if refused.iter().any(|name| name == row.key)
            && every_row
                .is_some_and(|cells| cells.iter().any(|cell| cell.as_str() == Some(row.key)))
        {
            self.problems.push(invalid(
                &self.input.at("everyRowMayImport"),
                &format!(
                    "`everyRowMayImport` names the row `{}`, and `mayNotImportItself` lists it; \
                     keep one of the two",
                    row.key
                ),
            ));
        }
        let Some(cells) = row.cells.as_array() else {
            self.problems.push(invalid(
                &field,
                "must be the list of what the row may import",
            ));
            return Side::default();
        };
        if refused.iter().any(|name| name == row.key)
            && cells.iter().any(|cell| cell.as_str() == Some(row.key))
        {
            self.problems.push(invalid(
                &field,
                &format!(
                    "the row `{}` names itself, and `mayNotImportItself` lists it; keep one of the \
                     two",
                    row.key
                ),
            ));
        }
        self.cells(cells, rows, &field)
    }
}

impl Shared {
    /// One row as one rule: a `mayOnlyImport` sentence, and a `mayNotImport` sentence of its own name
    /// when `mayNotImportItself` lists it.
    fn row_rule(&self, row: &Row<'_>, side: Side, refuses_itself: bool) -> Rule {
        let files = if self.default_files {
            row.files.clone()
        } else {
            self.files.and(&row.files)
        };
        let options = self.row_options(row, refuses_itself);
        let sentence = |condition| Sentence {
            files: files.clone(),
            folders: None,
            files_text: row.key.to_owned(),
            condition,
            options: options.clone(),
        };
        let mut sentences = vec![sentence(Condition::MayOnlyImport(side))];
        if refuses_itself {
            sentences.push(sentence(Condition::MayNotImport(Side {
                files: vec![row.files.clone()],
                text: row.key.to_owned(),
                ..Side::default()
            })));
        }
        self.rule(row.key, Some(row.key.to_owned()), sentences)
    }

    fn row_options(&self, row: &Row<'_>, refuses_itself: bool) -> ImportOptions {
        ImportOptions {
            within: Some(
                self.options
                    .within
                    .clone()
                    .unwrap_or_else(|| self.default_area.clone()),
            ),
            own: Some(OwnFiles {
                files: row.files.clone(),
                instance: row.instance,
                permitted: !refuses_itself,
            }),
            ..self.options.clone()
        }
    }

    /// The rule that a file in the table's files has a row: those files with no row must not exist.
    fn every_file_rule(&self, rows: &[Row<'_>]) -> Rule {
        let sets: Vec<FileSet> = rows.iter().map(|row| row.files.clone()).collect();
        let sentence = Sentence {
            files: self.files.and(&FileSet::any(&sets).not()),
            folders: None,
            files_text: format!("the files of {} that are in no row", self.full_name),
            condition: Condition::MustNotExist,
            options: ImportOptions::default(),
        };
        Rule {
            imports: ImportKinds::All,
            ..self.rule(EVERY_FILE_HAS_A_KIND, None, vec![sentence])
        }
    }

    fn rule(&self, name: &str, row: Option<String>, sentences: Vec<Sentence>) -> Rule {
        Rule {
            name: format!("{}.{name}", self.full_name),
            kind: RuleKind::TableRow {
                table: self.table,
                row,
            },
            sentences,
            reason: self.reason.clone(),
            because: self.because,
            link: self.link.clone(),
            imports: self.imports,
            examples: Examples::default(),
            source: self.source.clone(),
            replaces_preset_rule: false,
        }
    }
}

fn table_name<'v>(input: &Input<'v>, problems: &mut Vec<ConfigProblem>) -> Option<&'v str> {
    let Some(name) = input
        .object
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
    else {
        problems.push(invalid(
            &input.at("name"),
            "a table needs a name, because the name of each row starts with it",
        ));
        return None;
    };
    refuse_character(name, &input.at("name"), problems).then_some(name)
}

/// Whether a table name or a row key holds neither a colon nor a slash, which the exception comment
/// and a preset's prefix read.
fn refuse_character(name: &str, field: &str, problems: &mut Vec<ConfigProblem>) -> bool {
    if name.contains(':') {
        problems.push(invalid(
            field,
            &format!(
                "`{name}` has a colon, and an exception comment ends the rule name at the first \
                 colon, so no exception could name this rule"
            ),
        ));
        return false;
    }
    if name.contains('/') {
        problems.push(invalid(
            field,
            &format!(
                "`{name}` has a slash, and a slash separates a preset's name from the names it \
                 declares"
            ),
        ));
        return false;
    }
    true
}

fn unknown_name(name: &str, rows: &[&str], field: &str, names: &Names<'_>) -> ConfigProblem {
    let mut known: Vec<&str> = rows.to_vec();
    known.extend(names.elements.iter().map(|e| e.name.as_str()));
    known.extend(names.selectors.iter().map(String::as_str));
    ConfigProblem::new(
        ProblemCode::RuleUnknownElement,
        format!(
            "the table has no row \"{name}\", and the config declares no such name; {}the names are \
             {}; a path pattern has a `/` or a `*`",
            nearest(name, &known)
                .map(|near| format!("did you mean \"{near}\"? "))
                .unwrap_or_default(),
            known.join(", "),
        ),
    )
    .at(field)
}

fn join_text(text: &mut String, more: &str) {
    if !text.is_empty() {
        text.push_str(", ");
    }
    text.push_str(more);
}

/// A row's side, with the cells that every row has.
fn with_every_row(mut side: Side, every: &Side) -> Side {
    side.files.extend(every.files.iter().cloned());
    side.elements.extend(every.elements.iter().copied());
    side.packages.extend(every.packages.iter().cloned());
    if !every.text.is_empty() {
        join_text(&mut side.text, &every.text);
    }
    side
}

/// The area of a row without `within`: every repository file, and the externals sets that the table
/// names in a cell or in `everyRowMayImport`.
fn default_area(rows: &[Side], every: &Side) -> Side {
    let mut elements: Vec<ElementId> = Vec::new();
    for element in rows.iter().chain([every]).flat_map(|side| &side.elements) {
        if !elements.contains(element) {
            elements.push(*element);
        }
    }
    Side {
        files: vec![FileSet::every_file()],
        elements,
        packages: Vec::new(),
        text: "the repository".to_owned(),
    }
}

fn invalid(field: &str, message: &str) -> ConfigProblem {
    ConfigProblem::new(ProblemCode::ConfigInvalid, message).at(field)
}
