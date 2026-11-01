//! The tests of an import's own facts: the names it brings in, how its specifier is written, the
//! instances at its ends, and how the importing file's nearest manifest lists the package.

use archangel_config::{DependencyKind, InstanceMatch, Sentence, SpecifierForm};
use archangel_graph::{Import, ImportForm, Resolution};

use crate::architecture::Edge;

/// Whether every option of a sentence holds for an import.
pub(crate) fn hold(sentence: &Sentence, edge: &Edge<'_>) -> bool {
    let options = &sentence.options;
    options
        .names
        .as_ref()
        .is_none_or(|names| brings_in(edge.import, names))
        && options.specifier.is_none_or(|form| written_as(edge, form))
        && options
            .instance
            .is_none_or(|instance| instances_compare(sentence, edge, instance))
        && options
            .dependency
            .is_none_or(|kind| declared_as(edge, kind))
}

/// Whether both ends are in the element of `files`, in one instance or in two as asked.
fn instances_compare(sentence: &Sentence, edge: &Edge<'_>, instance: InstanceMatch) -> bool {
    let Some(element) = sentence.files.element() else {
        return false;
    };
    let (Some(a), Some(b)) = (
        edge.from.classification.and_then(|c| c.of(element)),
        edge.to.classification.and_then(|c| c.of(element)),
    ) else {
        return false;
    };
    (instance == InstanceMatch::Same) == (a.captures == b.captures)
}

/// Whether the importing file's nearest manifest lists the package as asked. Node's built-in
/// modules are never declared, so the option never applies to them.
fn declared_as(edge: &Edge<'_>, kind: DependencyKind) -> bool {
    let Some(package) = edge.to.package.filter(|p| !p.starts_with("node:")) else {
        return false;
    };
    let manifest = edge.manifests.and_then(|m| m.nearest(edge.file));
    let production = manifest.is_some_and(|m| m.dependencies.contains(package));
    let development = manifest.is_some_and(|m| m.development.contains(package));
    match kind {
        DependencyKind::Dev => development && !production,
        DependencyKind::Undeclared => !development && !production,
    }
}

/// Whether an import brings in one of the names. An import whose names archangel cannot see brings
/// in every name; a side-effect import brings in none.
fn brings_in(import: &Import, names: &[String]) -> bool {
    match import.form {
        ImportForm::Static if import.names.is_empty() => false,
        ImportForm::Static | ImportForm::ExportFrom => {
            import.names.is_empty()
                || import
                    .names
                    .iter()
                    .any(|name| name == "*" || names.contains(name))
        }
        ImportForm::Dynamic
        | ImportForm::Require
        | ImportForm::ImportEquals
        | ImportForm::ImportType
        | ImportForm::TripleSlashPath
        | ImportForm::TripleSlashTypes => true,
    }
}

/// Whether an import's specifier is written in a form.
fn written_as(edge: &Edge<'_>, form: SpecifierForm) -> bool {
    let specifier = edge.import.specifier.as_str();
    let parent = specifier == ".." || specifier.starts_with("../");
    let relative = parent || specifier == "." || specifier.starts_with("./");
    let package = is_package(edge, relative);
    match form {
        SpecifierForm::Relative => relative,
        SpecifierForm::Parent => parent,
        SpecifierForm::Package => package,
        SpecifierForm::Alias => {
            !relative
                && !specifier.starts_with('/')
                && !package
                && matches!(edge.resolution, Resolution::File(_))
        }
    }
}

/// Whether an import names a package: one in `node_modules`, or a package of the repository's own
/// workspace, which resolves to a repository file through its link.
fn is_package(edge: &Edge<'_>, relative: bool) -> bool {
    match edge.resolution {
        Resolution::Package(_) => true,
        Resolution::File(_) => {
            let specifier = edge.import.specifier.as_str();
            !relative
                && !specifier.starts_with('#')
                && edge
                    .manifests
                    .is_some_and(|m| m.is_workspace_package(&package_of(specifier)))
        }
        Resolution::Unresolved(_) => false,
    }
}

/// The package name of a bare specifier: `@scope/name` or `name`.
fn package_of(specifier: &str) -> String {
    let mut parts = specifier.split('/');
    match (parts.next(), parts.next()) {
        (Some(scope), Some(name)) if scope.starts_with('@') => format!("{scope}/{name}"),
        (Some(name), _) => name.to_owned(),
        (None, _) => String::new(),
    }
}
