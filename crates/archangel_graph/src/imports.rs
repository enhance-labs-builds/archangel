//! Finding the imports, exported names and exception comments of one source file.

use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, CallExpression, Declaration, Expression, ImportOrExportKind, Program, Statement,
    TSImportEqualsDeclaration, TSImportType, TSModuleReference,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxc_syntax::module_record::{
    ExportExportName, ExportImportName, ImportImportName, ModuleRecord,
};

/// The largest source the parser accepts, in bytes.
pub const PARSER_LIMIT_BYTES: u64 = u32::MAX as u64;

/// What one source file imports and exports.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileFacts {
    /// The imports, in the order they appear.
    pub imports: Vec<Import>,
    /// The names the file exports, and the specifiers of its `export *` statements.
    pub exports: ExportNames,
    /// Comments that contain `archangel-allow`, which may be exceptions.
    pub allow_comments: Vec<CommentText>,
    /// Lines of dynamic imports and require calls whose specifier is computed.
    pub unfollowed: Vec<u32>,
    /// The directive that is the file's first statement, such as `use client`, without quotes.
    pub directive: Option<String>,
    /// The first top-level statement that is the file's own: anything but an import, an export
    /// from another file, a directive, or an `export { … }` of names the file imported.
    pub own_statement: Option<OwnStatement>,
}

/// A top-level statement that declares or runs something of the file's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnStatement {
    /// The line, from 1.
    pub line: u32,
    /// The column, from 1, in characters.
    pub column: u32,
    /// The name it declares, when it declares one.
    pub name: Option<String>,
}

/// One import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// The text that names what is imported.
    pub specifier: String,
    /// Whether the import stays in the compiled JavaScript.
    pub kind: ImportKind,
    /// The form of the import.
    pub form: ImportForm,
    /// The line of the specifier, from 1.
    pub line: u32,
    /// The first line of the statement that holds the specifier, from 1.
    pub statement_line: u32,
    /// The column of the specifier, from 1, in characters.
    pub column: u32,
    /// The byte offsets of the specifier, quotes included.
    pub span: (u32, u32),
    /// The start and end of the specifier on its line, from 0, in UTF-16 code units, quotes
    /// included: the unit an editor's language server positions count in.
    pub utf16: (u32, u32),
    /// The names an import or export-from statement brings in: `default`, `*` or the name. An
    /// `export *` and a side-effect import list none.
    pub names: Vec<String>,
    /// Whether every name has its own `type` modifier and the statement has none, so that the
    /// tsconfig of the file decides the mark.
    pub names_typed: bool,
}

impl FileFacts {
    /// Sets the mark of each import that the file's tsconfig decides: `verbatimModuleSyntax` makes
    /// TypeScript keep a statement whose names all have the `type` modifier, so the module runs.
    pub(crate) fn mark_imports(&mut self, verbatim_module_syntax: bool) {
        for import in self.imports.iter_mut().filter(|import| import.names_typed) {
            import.kind = if verbatim_module_syntax {
                ImportKind::Runtime
            } else {
                ImportKind::Type
            };
        }
    }
}

/// Whether an import stays in the compiled JavaScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImportKind {
    /// It stays.
    Runtime,
    /// The compiler removes it.
    Type,
}

/// The syntax an import uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportForm {
    /// `import ... from '...'` or `import '...'`.
    Static,
    /// `export ... from '...'`.
    ExportFrom,
    /// `import('...')`.
    Dynamic,
    /// `require('...')`.
    Require,
    /// `import x = require('...')`.
    ImportEquals,
    /// `import('...').Name` in a type.
    ImportType,
    /// `/// <reference path="..." />`.
    TripleSlashPath,
    /// `/// <reference types="..." />`.
    TripleSlashTypes,
}

/// What a file exports.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportNames {
    /// The names it exports, `default` included.
    pub names: Vec<String>,
    /// The specifiers of its `export * from '...'` statements.
    pub star_from: Vec<String>,
    /// Each export, in the order of the file.
    pub entries: Vec<Export>,
}

/// One export of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    /// The name it exports: `default`, a name, or `*` for an `export *` that names nothing.
    pub name: String,
    /// Whether it is an `export *` or an `export * as`.
    pub star: bool,
    /// The line, from 1.
    pub line: u32,
    /// The column, from 1, in characters.
    pub column: u32,
}

/// The text of a comment, with the lines it starts and ends on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentText {
    /// The first line, from 1.
    pub line: u32,
    /// The last line, from 1.
    pub end_line: u32,
    /// The comment's text without `//` or `/* */`.
    pub text: String,
}

/// Why a file gave no facts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParseFailure {
    /// Every error the parser found.
    pub errors: Vec<ParseError>,
    /// The file uses Flow syntax, which the parser does not read.
    pub flow: bool,
    /// The file is larger than the parser accepts.
    pub too_large: bool,
}

/// One syntax error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The line, from 1.
    pub line: u32,
    /// The column, from 1.
    pub column: u32,
    /// The parser's description.
    pub message: String,
}

/// Parses one file with its own allocator.
///
/// # Errors
///
/// Returns every syntax error when the file does not parse.
pub fn extract_file(path: &str, text: &str) -> Result<FileFacts, ParseFailure> {
    extract(&Allocator::default(), path, text)
}

/// Parses one file into its facts, with an allocator that the caller reuses.
///
/// # Errors
///
/// Returns every syntax error when the file does not parse.
pub fn extract(allocator: &Allocator, path: &str, text: &str) -> Result<FileFacts, ParseFailure> {
    let source_type = source_type(path);
    let source = Source {
        lines: LineIndex::new(text),
        text,
        declaration: source_type.is_typescript_definition(),
    };
    let parsed = Parser::new(allocator, text, source_type).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        return Err(source.failure(&parsed.errors, parsed.is_flow_language));
    }
    let record = &parsed.module_record;
    let mut facts = FileFacts::default();
    source.static_imports(record, &mut facts);
    source.dynamic_imports(record, &mut facts);
    if memchr::memmem::find(text.as_bytes(), b"require").is_some()
        || memchr::memmem::find(text.as_bytes(), b"import(").is_some()
    {
        let mut walker = Walker {
            source: &source,
            facts: &mut facts,
        };
        walker.visit_program(&parsed.program);
    }
    source.comments(&parsed.program, &mut facts);
    facts.directive = parsed
        .program
        .directives
        .first()
        .map(|directive| directive.expression.value.to_string());
    source.exports(record, &mut facts.exports);
    facts.own_statement = source.own_statement(&parsed.program, record);
    facts.imports.sort_by_key(|import| import.span);
    facts.unfollowed.sort_unstable();
    Ok(facts)
}

fn source_type(path: &str) -> SourceType {
    let base = SourceType::from_path(Path::new(path)).unwrap_or_default();
    let javascript = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e, "js" | "mjs" | "cjs"));
    if javascript {
        base.with_jsx(true)
    } else {
        base
    }
}

/// One file's text, with what every step of the extraction needs to know about it.
struct Source<'t> {
    lines: LineIndex,
    text: &'t str,
    declaration: bool,
}

impl Source<'_> {
    /// An import at a span; a runtime import unless the file is a declaration file.
    fn import(&self, specifier: &str, span: Span, form: ImportForm) -> Import {
        let (line, column) = self.position(span.start);
        Import {
            specifier: specifier.to_owned(),
            kind: if self.declaration {
                ImportKind::Type
            } else {
                ImportKind::Runtime
            },
            form,
            line,
            statement_line: line,
            column,
            span: (span.start, span.end),
            utf16: self.lines.utf16(self.text, span.start, span.end),
            names: Vec::new(),
            names_typed: false,
        }
    }

    fn position(&self, offset: u32) -> (u32, u32) {
        self.lines.position(self.text, offset)
    }

    fn slice(&self, span: Span) -> &str {
        self.text
            .get(span.start as usize..span.end as usize)
            .unwrap_or_default()
    }

    fn failure(&self, errors: &[oxc_diagnostics::OxcDiagnostic], flow: bool) -> ParseFailure {
        let errors = errors
            .iter()
            .map(|error| {
                let start = error
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.first())
                    .map_or(0, |label| u32::try_from(label.offset()).unwrap_or(u32::MAX));
                let (line, column) = self.position(start);
                ParseError {
                    line,
                    column,
                    message: error.message.to_string(),
                }
            })
            .collect();
        ParseFailure {
            errors,
            flow,
            too_large: false,
        }
    }

    fn static_imports(&self, record: &ModuleRecord<'_>, facts: &mut FileFacts) {
        for (specifier, requests) in &record.requested_modules {
            for request in requests {
                let form = if request.is_import {
                    ImportForm::Static
                } else {
                    ImportForm::ExportFrom
                };
                let statement = StatementNames::of(record, request.statement_span);
                let mut import = self
                    .import(specifier.as_str(), request.span, form)
                    .typed(request.is_type || statement.all_typed)
                    .with_names(statement.names);
                import.names_typed = statement.all_typed && !request.is_type && !self.declaration;
                import.statement_line = self.position(request.statement_span.start).0;
                facts.imports.push(import);
            }
        }
    }

    fn dynamic_imports(&self, record: &ModuleRecord<'_>, facts: &mut FileFacts) {
        for dynamic in &record.dynamic_imports {
            match literal(self.slice(dynamic.module_request)) {
                Some(specifier) => facts.imports.push(self.import(
                    &specifier,
                    dynamic.module_request,
                    ImportForm::Dynamic,
                )),
                None => facts.unfollowed.push(self.position(dynamic.span.start).0),
            }
        }
    }

    fn comments(&self, program: &Program<'_>, facts: &mut FileFacts) {
        let first_statement = program.body.first().map_or(u32::MAX, |statement| {
            oxc_span::GetSpan::span(statement).start
        });
        for comment in &program.comments {
            let body = self.slice(comment.content_span());
            if comment.span.end <= first_statement
                && let Some((form, specifier)) = triple_slash(body)
            {
                let start = comment.content_span().start;
                facts.imports.push(
                    self.import(&specifier, Span::new(start, start), form)
                        .typed(true),
                );
            }
            if body.contains("archangel-allow") {
                facts.allow_comments.push(CommentText {
                    line: self.position(comment.span.start).0,
                    end_line: self.position(comment.span.end).0,
                    text: body.to_owned(),
                });
            }
        }
    }
}

impl Import {
    /// The same import, as a type import when `is_type` holds.
    fn typed(mut self, is_type: bool) -> Self {
        if is_type {
            self.kind = ImportKind::Type;
        }
        self
    }

    fn with_names(mut self, names: Vec<String>) -> Self {
        self.names = names;
        self
    }
}

/// What the import or export-from statement at one span names.
struct StatementNames {
    names: Vec<String>,
    all_typed: bool,
}

impl StatementNames {
    fn of(record: &ModuleRecord<'_>, statement: Span) -> Self {
        let imports: Vec<_> = record
            .import_entries
            .iter()
            .filter(|e| e.statement_span == statement)
            .collect();
        let reexports: Vec<_> = record
            .indirect_export_entries
            .iter()
            .filter(|e| e.statement_span == statement)
            .collect();
        let all_typed = (!imports.is_empty() && imports.iter().all(|e| e.is_type))
            || (!reexports.is_empty() && reexports.iter().all(|e| e.is_type));
        let reexported = reexports
            .iter()
            .filter_map(|entry| match &entry.import_name {
                ExportImportName::Name(name) => Some(name.name.to_string()),
                ExportImportName::All | ExportImportName::AllButDefault => Some("*".to_owned()),
                ExportImportName::Null => None,
            });
        Self {
            names: imports
                .iter()
                .map(|entry| import_name(&entry.import_name))
                .chain(reexported)
                .collect(),
            all_typed,
        }
    }
}

/// The value of a string literal, or of a template literal without substitutions.
fn literal(expression: &str) -> Option<String> {
    let trimmed = expression.trim();
    let quote = trimmed.chars().next()?;
    let inner = trimmed.get(1..trimmed.len().checked_sub(1)?)?;
    let closed = trimmed.len() >= 2 && trimmed.ends_with(quote);
    let plain = match quote {
        '\'' | '"' => true,
        '`' => !inner.contains("${"),
        _ => false,
    };
    (closed && plain && !inner.contains('\\')).then(|| inner.to_owned())
}

struct Walker<'f, 't> {
    source: &'f Source<'t>,
    facts: &'f mut FileFacts,
}

impl<'a> Visit<'a> for Walker<'_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = &call.callee
            && callee.name == "require"
            && call.arguments.len() == 1
        {
            match &call.arguments[0] {
                Argument::StringLiteral(literal) => {
                    let import = self.source.import(
                        literal.value.as_str(),
                        literal.span,
                        ImportForm::Require,
                    );
                    self.facts.imports.push(import);
                }
                _ => self
                    .facts
                    .unfollowed
                    .push(self.source.position(call.span.start).0),
            }
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_ts_import_equals_declaration(&mut self, declaration: &TSImportEqualsDeclaration<'a>) {
        if let TSModuleReference::ExternalModuleReference(reference) = &declaration.module_reference
        {
            let target = &reference.expression;
            let import = self
                .source
                .import(target.value.as_str(), target.span, ImportForm::ImportEquals)
                .typed(declaration.import_kind == ImportOrExportKind::Type);
            self.facts.imports.push(import);
        }
        walk::walk_ts_import_equals_declaration(self, declaration);
    }

    fn visit_ts_import_type(&mut self, import_type: &TSImportType<'a>) {
        let target = &import_type.source;
        let import = self
            .source
            .import(target.value.as_str(), target.span, ImportForm::ImportType)
            .typed(true);
        self.facts.imports.push(import);
        walk::walk_ts_import_type(self, import_type);
    }
}

/// `/ <reference path="x" />` from a `///` comment, whose content starts with the third slash.
fn triple_slash(body: &str) -> Option<(ImportForm, String)> {
    let rest = body
        .strip_prefix('/')?
        .trim_start()
        .strip_prefix("<reference")?;
    for (attribute, form) in [
        ("path", ImportForm::TripleSlashPath),
        ("types", ImportForm::TripleSlashTypes),
    ] {
        let marker = format!("{attribute}=");
        if let Some(start) = rest.find(&marker) {
            let value = &rest[start + marker.len()..];
            let quote = value.chars().next()?;
            let value = value.get(1..)?;
            let end = value.find(quote)?;
            return Some((form, value[..end].to_owned()));
        }
    }
    None
}

impl Source<'_> {
    fn exports(&self, record: &ModuleRecord<'_>, exports: &mut ExportNames) {
        for entry in record
            .local_export_entries
            .iter()
            .chain(&record.indirect_export_entries)
        {
            let name = match &entry.export_name {
                ExportExportName::Name(name) => name.name.to_string(),
                ExportExportName::Default(_) => "default".to_owned(),
                ExportExportName::Null => continue,
            };
            let star = matches!(
                entry.import_name,
                ExportImportName::All | ExportImportName::AllButDefault
            );
            exports
                .entries
                .push(self.export(name.clone(), star, entry.statement_span));
            exports.names.push(name);
        }
        for entry in &record.star_export_entries {
            if let Some(request) = &entry.module_request {
                exports.star_from.push(request.name.to_string());
            }
            exports
                .entries
                .push(self.export("*".to_owned(), true, entry.statement_span));
        }
        exports.names.sort();
        exports.names.dedup();
        exports.entries.sort_by_key(|e| (e.line, e.column));
    }

    fn export(&self, name: String, star: bool, span: Span) -> Export {
        let (line, column) = self.position(span.start);
        Export {
            name,
            star,
            line,
            column,
        }
    }

    /// The first top-level statement that is the file's own. Only the top level is read, so this
    /// costs one look at each statement and no walk of the tree.
    fn own_statement(
        &self,
        program: &Program<'_>,
        record: &ModuleRecord<'_>,
    ) -> Option<OwnStatement> {
        let imported: Vec<&str> = record
            .import_entries
            .iter()
            .map(|entry| entry.local_name.name.as_str())
            .collect();
        program.body.iter().find_map(|statement| {
            let name = match statement {
                Statement::ImportDeclaration(_)
                | Statement::ExportAllDeclaration(_)
                | Statement::EmptyStatement(_) => return None,
                Statement::TSImportEqualsDeclaration(declaration)
                    if matches!(
                        declaration.module_reference,
                        TSModuleReference::ExternalModuleReference(_)
                    ) =>
                {
                    return None;
                }
                Statement::ExportNamedDeclaration(export) => {
                    if export.source.is_some() {
                        return None;
                    }
                    if let Some(declaration) = &export.declaration {
                        declared(declaration)
                    } else {
                        let local = export
                            .specifiers
                            .iter()
                            .map(|specifier| specifier.local.name())
                            .find(|local| !imported.contains(&local.as_str()))?;
                        Some(local.to_string())
                    }
                }
                Statement::ExportDefaultDeclaration(_) => Some("default".to_owned()),
                other => other.as_declaration().and_then(declared),
            };
            let (line, column) = self.position(oxc_span::GetSpan::span(statement).start);
            Some(OwnStatement { line, column, name })
        })
    }
}

/// The name a declaration binds: the first name of a variable declaration.
fn declared(declaration: &Declaration<'_>) -> Option<String> {
    match declaration {
        Declaration::VariableDeclaration(variables) => variables
            .declarations
            .first()
            .and_then(|variable| variable.id.get_identifier_name())
            .map(|name| name.to_string()),
        other => other.id().map(|id| id.name.to_string()),
    }
}

fn import_name(name: &ImportImportName<'_>) -> String {
    match name {
        ImportImportName::Name(name) => name.name.to_string(),
        ImportImportName::NamespaceObject => "*".to_owned(),
        ImportImportName::Default(_) => "default".to_owned(),
    }
}

/// Byte offsets of the line starts, to turn an offset into a line and a column.
struct LineIndex {
    starts: Vec<u32>,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(
            memchr::memchr_iter(b'\n', text.as_bytes())
                .map(|i| u32::try_from(i + 1).unwrap_or(u32::MAX)),
        );
        Self { starts }
    }

    /// The line from 1 and the column from 1, in characters.
    fn position(&self, text: &str, offset: u32) -> (u32, u32) {
        let line = self.starts.partition_point(|start| *start <= offset);
        let start = self.line_start(line);
        let column = text
            .get(start as usize..offset as usize)
            .map_or(0, |prefix| prefix.chars().count());
        (
            u32::try_from(line).unwrap_or(u32::MAX),
            u32::try_from(column + 1).unwrap_or(u32::MAX),
        )
    }
    /// The start and end of a span on the line it starts on, from 0, in UTF-16 code units.
    fn utf16(&self, text: &str, start: u32, end: u32) -> (u32, u32) {
        let line_start = self.line_start(self.starts.partition_point(|s| *s <= start));
        let units = |from: u32, to: u32| {
            text.get(from as usize..to as usize)
                .map_or(0, |part| part.encode_utf16().count())
        };
        let before = units(line_start, start);
        let width = units(start, end);
        (
            u32::try_from(before).unwrap_or(u32::MAX),
            u32::try_from(before + width).unwrap_or(u32::MAX),
        )
    }

    /// The byte offset of a line's start, for a line from 1.
    fn line_start(&self, line: usize) -> u32 {
        self.starts
            .get(line.saturating_sub(1))
            .copied()
            .unwrap_or(0)
    }
}
