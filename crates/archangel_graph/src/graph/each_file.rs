//! Resolving the imports of one file: its tsconfig, each import as the file writes it, the names of
//! the package modules that imports of every name bring in, and the files that a template import
//! reaches.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::Resolved;
use crate::imports::{self, FileFacts};
use crate::resolve::{
    ImportResolver, Importer, Resolution, ResolutionFileProblem, ResolutionInputs, UnresolvedReason,
};

/// The resolutions of one file's imports, with what decides them.
pub(super) fn resolve_file(
    resolver: &ImportResolver,
    file: &Path,
    facts: Option<&FileFacts>,
) -> Resolved {
    let mut problems = BTreeSet::new();
    let mut inputs = ResolutionInputs::default();
    let Some(facts) = facts else {
        return Resolved {
            problems,
            inputs,
            ..Resolved::default()
        };
    };
    let (tsconfig, problem) = resolver.tsconfig(file);
    problems.extend(problem);
    // A tsconfig that appears in a folder nearer than the one in use, or the first one anywhere,
    // decides for this file from then on.
    let found = tsconfig.as_ref().map(|found| found.path.as_path());
    inputs.existence.extend(
        file.ancestors()
            .skip(1)
            .map(|folder| folder.join("tsconfig.json"))
            .take_while(|candidate| Some(candidate.as_path()) != found),
    );
    if let Some(tsconfig) = &tsconfig {
        inputs.contents.insert(tsconfig.path.clone());
        inputs
            .contents
            .extend(tsconfig.references_resolved.iter().map(|r| r.path.clone()));
    }
    let verbatim_module_syntax = tsconfig
        .as_ref()
        .is_some_and(|found| found.compiler_options.verbatim_module_syntax == Some(true));
    let importer = Importer {
        file,
        tsconfig: tsconfig.as_deref(),
    };
    let importing = Importing {
        resolver,
        file,
        importer,
        verbatim_module_syntax,
    };
    let each = EachImport::of(&importing, &facts.imports, &mut inputs, &mut problems);
    Resolved {
        problems,
        inputs,
        verbatim_module_syntax,
        ..each.into_resolved()
    }
}

/// Whether the compiler removes an import: a type import, or one whose names are all typed
/// without `verbatimModuleSyntax`.
fn removed_by_the_compiler(import: &imports::Import, verbatim_module_syntax: bool) -> bool {
    if import.names_typed {
        !verbatim_module_syntax
    } else {
        import.kind == imports::ImportKind::Type
    }
}

/// The resolution of each import of a file, with the imports that a template import stands for.
#[derive(Default)]
struct EachImport {
    resolutions: Vec<Resolution>,
    expanded: Vec<imports::Import>,
    module_names: Vec<Option<Arc<[String]>>>,
    modules_parsed: u64,
}

/// What resolving one file's imports needs.
struct Importing<'a> {
    resolver: &'a ImportResolver,
    file: &'a Path,
    importer: Importer<'a>,
    verbatim_module_syntax: bool,
}

impl EachImport {
    /// Resolves each import of a file as the file writes it.
    fn of(
        importing: &Importing<'_>,
        imports: &[imports::Import],
        inputs: &mut ResolutionInputs,
        problems: &mut BTreeSet<ResolutionFileProblem>,
    ) -> Self {
        let resolver = importing.resolver;
        let mut each = Self::default();
        for import in written_imports(imports) {
            if let Some(pattern) = &import.template {
                each.template(import, &resolver.template_files(importing.file, pattern));
                continue;
            }
            let removed = removed_by_the_compiler(import, importing.verbatim_module_syntax);
            let (resolution, problem, package_file) =
                resolver.resolve_import(&importing.importer, (&import.specifier, removed), inputs);
            problems.extend(problem);
            let (names, parsed) = names_brought_in(resolver, import, package_file, inputs);
            each.push(import, resolution, names);
            each.modules_parsed += parsed;
        }
        each
    }

    /// The resolutions, with the imports when a template import stands for several.
    fn into_resolved(self) -> Resolved {
        let has_template = self.expanded.iter().any(|import| import.template.is_some());
        Resolved {
            resolutions: self.resolutions,
            imports: has_template.then_some(self.expanded),
            module_names: self.module_names,
            modules_parsed: self.modules_parsed,
            ..Resolved::default()
        }
    }

    fn push(
        &mut self,
        import: &imports::Import,
        resolution: Resolution,
        names: Option<Arc<[String]>>,
    ) {
        self.resolutions.push(resolution);
        self.expanded.push(import.clone());
        self.module_names.push(names);
    }

    /// One import of each file that a template reaches, or one unresolved import when it reaches
    /// none.
    fn template(&mut self, import: &imports::Import, reached: &[String]) {
        if reached.is_empty() {
            let nothing = Resolution::Unresolved(UnresolvedReason::TemplateMatchesNothing);
            self.push(import, nothing, None);
        }
        for path in reached {
            self.push(import, Resolution::File(path.clone()), None);
        }
    }
}

/// The imports as the file writes them: a template import that an earlier resolution made one
/// import of each file it reaches counts once.
fn written_imports(imports: &[imports::Import]) -> impl Iterator<Item = &imports::Import> {
    imports
        .iter()
        .enumerate()
        .filter_map(move |(index, import)| {
            let copy = import.template.is_some()
                && index > 0
                && imports[index - 1].template.is_some()
                && imports[index - 1].span == import.span;
            (!copy).then_some(import)
        })
}

/// The names of the package module that an import of every name brings in, and the package files
/// parsed to find them.
fn names_brought_in(
    resolver: &ImportResolver,
    import: &imports::Import,
    package_file: Option<PathBuf>,
    inputs: &mut ResolutionInputs,
) -> (Option<Arc<[String]>>, u64) {
    match package_file {
        Some(module) if import.brings_in_every_name() => resolver.module_names(&module, inputs),
        _ => (None, 0),
    }
}
