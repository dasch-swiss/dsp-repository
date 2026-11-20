//! The public guard: whether the anonymous user may view every selected resource
//! and value, decided from each one's own `kb:hasPermissions` in the data graph.
//! It fails closed: a missing or unreadable permission is a violation, and only a
//! grant naming `knora-admin:UnknownUser` counts.

use std::collections::BTreeMap;
use std::fmt;

use oxigraph::model::Term;

use crate::model::{self, kb};
use crate::selection::Selection;
use crate::snapshot::index::{Entry, Index};
use crate::snapshot::kinds::{self, Subjects};

/// The two spellings of the anonymous user's group in a permission literal.
const UNKNOWN_USER: &[&str] = &[
    "knora-admin:UnknownUser",
    "http://www.knora.org/ontology/knora-admin#UnknownUser",
];

/// A DSP-API permission token, weakest first, so the derived order compares grants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Token {
    /// `RV`, restricted view.
    RestrictedView,
    /// `V`, view.
    View,
    /// `M`, modify.
    Modify,
    /// `D`, delete.
    Delete,
    /// `CR`, change rights.
    ChangeRights,
}

impl Token {
    fn parse(s: &str) -> Option<Token> {
        Some(match s {
            "RV" => Token::RestrictedView,
            "V" => Token::View,
            "M" => Token::Modify,
            "D" => Token::Delete,
            "CR" => Token::ChangeRights,
            _ => return None,
        })
    }
}

/// Whether a violating subject is a resource or a value (a value, file value or
/// LinkValue node).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubjectKind {
    /// A resource: its class is a project class, `kb:Region` or `kb:LinkObj`.
    Resource,
    /// A value, file value or LinkValue node.
    Value,
}

/// Why the anonymous user may not view a subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Problem {
    /// No grant to `knora-admin:UnknownUser`.
    NotViewable,
    /// Restricted view on anything but a raster still image's file value.
    RestrictedViewNotAllowed,
    /// No `kb:hasPermissions`.
    MissingPermissions,
    /// A `kb:hasPermissions` that is not one string of known tokens and groups.
    MalformedPermissions,
}

impl Problem {
    fn as_str(self) -> &'static str {
        match self {
            Problem::NotViewable => "not viewable by the anonymous user",
            Problem::RestrictedViewNotAllowed => "restricted view on something other than a raster still image",
            Problem::MissingPermissions => "no knora-base:hasPermissions",
            Problem::MalformedPermissions => "malformed knora-base:hasPermissions",
        }
    }
}

/// One selected resource or value the anonymous user may not view. It names the
/// subject and its class, never its content or its permission literal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    /// The subject's IRI.
    pub subject: String,
    pub kind: SubjectKind,
    /// The subject's `rdf:type`.
    pub class: String,
    pub problem: Problem,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            SubjectKind::Resource => "resource",
            SubjectKind::Value => "value",
        };
        write!(f, "{} ({kind} {}): {}", self.subject, self.class, self.problem.as_str())
    }
}

/// The access rights the format records (`FORMAT.md` §2, §8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AccessRights {
    Public,
    RestrictedView,
}

impl AccessRights {
    /// The literal written as `dao:accessRights`.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            AccessRights::Public => "public",
            AccessRights::RestrictedView => "restricted-view",
        }
    }
}

/// The access rights of every subject the guard passed, and of nothing else: a
/// subject the guard did not pass has none, so it can never be written as public.
#[derive(Debug)]
pub(crate) struct Rights(BTreeMap<String, AccessRights>);

impl Rights {
    pub(crate) fn of(&self, iri: &str) -> Option<AccessRights> {
        self.0.get(iri).copied()
    }

    /// Every selected subject public, as if the guard had passed them all: for the
    /// snapshot tests, whose fixtures carry no `kb:hasPermissions`.
    #[cfg(test)]
    pub(crate) fn all_public(selection: &Selection) -> Rights {
        let all = selection.resources().chain(selection.values());
        Rights(all.map(|iri| (iri.to_string(), AccessRights::Public)).collect())
    }
}

/// Check every selected resource and value: the rights of those that pass, and a
/// violation for each that does not, sorted. The caller refuses on any violation.
pub(crate) fn check(index: &Index, subjects: &Subjects, selection: &Selection) -> (Rights, Vec<Violation>) {
    let mut rights = Rights(BTreeMap::new());
    let mut violations = Vec::new();
    let selected = selection
        .resources()
        .map(|iri| (iri, SubjectKind::Resource))
        .chain(selection.values().map(|iri| (iri, SubjectKind::Value)));
    for (iri, kind) in selected {
        let class = subjects
            .get(iri)
            .and_then(|s| s.class.as_ref())
            .map(|c| c.as_str())
            .unwrap_or_default();
        let grant = index.get(iri).map_or(Err(Problem::MissingPermissions), anonymous_grant);
        let problem = match grant {
            Ok(Some(token)) if token >= Token::View => {
                rights.0.insert(iri.to_string(), AccessRights::Public);
                continue;
            }
            Ok(Some(_)) if kind == SubjectKind::Value && class == kb::STILL_IMAGE_FILE_VALUE => {
                rights.0.insert(iri.to_string(), AccessRights::RestrictedView);
                continue;
            }
            Ok(Some(_)) => Problem::RestrictedViewNotAllowed,
            Ok(None) => Problem::NotViewable,
            Err(problem) => problem,
        };
        violations.push(Violation {
            subject: iri.to_string(),
            kind,
            class: class.to_string(),
            problem,
        });
    }
    violations.sort();
    (rights, violations)
}

/// The best token the subject's one `kb:hasPermissions` grants the anonymous user.
fn anonymous_grant(entry: &Entry) -> Result<Option<Token>, Problem> {
    let literal = match kinds::objects(entry, kb::HAS_PERMISSIONS).as_slice() {
        [] => return Err(Problem::MissingPermissions),
        [Term::Literal(l)] if l.language().is_none() && l.datatype().as_str() == model::XSD_STRING => l.value(),
        _ => return Err(Problem::MalformedPermissions),
    };
    parse_anonymous_grant(literal)
}

/// `<TOKEN> <group>[,<group>...]` entries joined by `|`, in any order. Every entry
/// must parse; the result is the highest token among the entries naming the
/// anonymous user.
fn parse_anonymous_grant(literal: &str) -> Result<Option<Token>, Problem> {
    let mut best = None;
    for entry in literal.split('|') {
        let (token, groups) = entry.split_once(' ').ok_or(Problem::MalformedPermissions)?;
        let token = Token::parse(token).ok_or(Problem::MalformedPermissions)?;
        let groups: Vec<&str> = groups.split(',').collect();
        if groups.iter().any(|g| g.is_empty()) {
            return Err(Problem::MalformedPermissions);
        }
        if groups.iter().any(|g| UNKNOWN_USER.contains(g)) {
            best = best.max(Some(token));
        }
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_highest_token_naming_the_anonymous_user_wins_in_any_order() {
        let both = [
            "RV knora-admin:UnknownUser|V knora-admin:KnownUser,knora-admin:UnknownUser",
            "V knora-admin:KnownUser,knora-admin:UnknownUser|RV knora-admin:UnknownUser",
        ];
        for literal in both {
            assert_eq!(parse_anonymous_grant(literal), Ok(Some(Token::View)), "{literal}");
        }
    }

    #[test]
    fn the_full_group_iri_counts_and_other_groups_do_not() {
        let full = "V http://www.knora.org/ontology/knora-admin#UnknownUser";
        assert_eq!(parse_anonymous_grant(full), Ok(Some(Token::View)));
        let others = "CR knora-admin:Creator|V knora-admin:KnownUser,knora-admin:ProjectMember";
        assert_eq!(parse_anonymous_grant(others), Ok(None));
    }

    #[test]
    fn an_entry_without_a_group_an_empty_group_or_an_unknown_token_is_malformed() {
        for literal in [
            "",
            "V",
            "V knora-admin:UnknownUser|",
            "V knora-admin:UnknownUser,",
            "X knora-admin:UnknownUser",
            "v knora-admin:UnknownUser",
        ] {
            assert_eq!(
                parse_anonymous_grant(literal),
                Err(Problem::MalformedPermissions),
                "{literal:?}"
            );
        }
    }
}
