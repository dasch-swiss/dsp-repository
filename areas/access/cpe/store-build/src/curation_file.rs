//! A project's curation file, read into the curated values [`crate::build`] takes.
//!
//! The file is UTF-8 text, one record per line, each line ended by a line feed; the last may lack
//! it. Line 1 is the header: `iri`, then one column per curated key, named `key` or `key@lang`
//! with both parts curation names ([`is_curation_name`]), or a comment column, whose name
//! starts with `#` and whose cells are never served. No two columns share a name. Every later line
//! is one resource of the snapshot: its full IRI, then exactly one cell per column. No IRI has two
//! rows, and no line is blank.
//!
//! A cell is quoted exactly when it starts with `"`. It then runs to the next quote that is not
//! doubled, which a comma or the end of the line follows; a quote anywhere else is refused, so no
//! cell holds a line break. A cell's value is what the quotes hold. An empty value is absent. Any
//! other is served verbatim, and refused, never trimmed, where it starts or ends with white space,
//! U+200B or U+FEFF. A control character, U+2028 or U+2029 is refused wherever it stands.
//!
//! The reader gives no key a meaning: no code here may name a project's key.

use std::collections::{BTreeMap, BTreeSet};

use cpe_ports::{ProjectSnapshot, ResourceIri};

/// One curated value of one resource. The reader gives `key` no meaning: the project's KDL and
/// hook do. `Ord` is resource, then key, then language (none first), then text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CuratedValue {
    pub resource: ResourceIri,
    /// The project's own name for the value; a curation name, `[a-z][a-z0-9_-]*`.
    pub key: String,
    /// The value's language, `None` where the project tags none; a curation name where there is
    /// one.
    pub lang: Option<String>,
    /// As [`CuratedValues::parse`] reads it: never empty, unpadded, without a control character.
    pub text: String,
}

/// Whether `name` can be a curated key or language: `[a-z][a-z0-9_-]*`, so `key@lang` reads one
/// way. A language tag with an upper-case part, such as `de-CH`, is refused by design.
pub(crate) fn is_curation_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// The curated values of one project, sorted. Production code makes them only by [`Self::parse`]
/// or [`Self::none`], so every value names a resource of the snapshot it was read against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuratedValues(Vec<CuratedValue>);

impl CuratedValues {
    /// The curated values of a curation file's bytes, each on a resource of `snapshot`, or the
    /// first fault met. Pure: no I/O.
    ///
    /// Faults are met in this order: bytes that are not UTF-8, wherever they stand; then line by
    /// line. Within a line, a refused character or a misplaced quote comes first, from left to
    /// right. The header's columns then follow from left to right. A row is then blank, of the
    /// wrong cell count, for an unknown IRI or the IRI's second, in that order, and only then
    /// holds a padded value, the leftmost first.
    pub fn parse(bytes: &[u8], snapshot: &ProjectSnapshot) -> Result<Self, CurationFault> {
        let known = snapshot.resources.iter().map(|resource| resource.iri.as_str()).collect();
        parse(bytes, &known).map(Self)
    }

    /// No curated values: the curation of a project that has no curation file.
    #[must_use]
    pub fn none() -> Self {
        Self(Vec::new())
    }

    /// `values` as they are, unchecked and unsorted, for tests that build curation the parser
    /// refuses.
    #[doc(hidden)]
    #[must_use]
    pub fn from_unchecked(values: Vec<CuratedValue>) -> Self {
        Self(values)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[CuratedValue] {
        &self.0
    }
}

/// A fault of a curation file. `line` is 1-based; the header is line 1.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {reason}")]
pub struct CurationFault {
    pub line: usize,
    pub reason: InvalidCuration,
}

/// The rule a curation file breaks.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidCuration {
    #[error("bytes that are not UTF-8")]
    NotUtf8,
    /// The file has no line, or its first line is empty.
    #[error("no header")]
    MissingHeader,
    /// A control character other than the line feed that ends a line, or U+2028 or U+2029.
    #[error("the control or line-break character {character:?}")]
    ControlCharacter { character: char },
    #[error("a quote inside a bare cell, or text after a closing quote")]
    StrayQuote,
    /// The fault's line is the one the quote opens on.
    #[error("a quoted cell that does not end on its line")]
    UnterminatedQuote,
    #[error("a first column named {found:?}, not iri")]
    FirstColumnNotIri { found: String },
    #[error("a column named {column:?}, which is neither key, key@lang nor a comment")]
    MalformedColumn { column: String },
    #[error("the column {column:?} twice")]
    DuplicateColumn { column: String },
    #[error("a blank line")]
    BlankLine,
    #[error("a row of {found} cells under a header of {expected}")]
    WrongCellCount { expected: usize, found: usize },
    #[error("a row for {iri:?}, which is not a resource of the snapshot")]
    UnknownResource { iri: String },
    /// `first_line` is the line of the IRI's first row.
    #[error("a second row for {iri:?}, after line {first_line}")]
    DuplicateRow { iri: String, first_line: usize },
    /// The value starts or ends with white space, U+200B or U+FEFF.
    #[error("a value under {column} that starts or ends with white space or an invisible character")]
    PaddedValue { column: String },
}

/// The values of [`CuratedValues::parse`], with `known` the IRIs of the snapshot's resources.
pub(crate) fn parse(bytes: &[u8], known: &BTreeSet<&str>) -> Result<Vec<CuratedValue>, CurationFault> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        let line_feeds = bytes[..error.valid_up_to()].iter().filter(|byte| **byte == b'\n').count();
        CurationFault { line: line_feeds + 1, reason: InvalidCuration::NotUtf8 }
    })?;
    // Not `str::lines`, which would swallow the carriage return `cells` must refuse.
    let mut lines = text.split_terminator('\n').enumerate().map(|(index, line)| (index + 1, line));

    let fault = |line: usize| move |reason: InvalidCuration| CurationFault { line, reason };
    let columns = match lines.next() {
        Some((line, header)) if !header.is_empty() => header_columns(header).map_err(fault(line))?,
        _ => return Err(fault(1)(InvalidCuration::MissingHeader)),
    };

    let mut values = Vec::new();
    let mut rows = BTreeMap::new();
    for (line, row) in lines {
        read_row(line, row, &columns, known, &mut rows, &mut values).map_err(fault(line))?;
    }
    values.sort();
    Ok(values)
}

/// The header's column names, the first being `iri`.
fn header_columns(header: &str) -> Result<Vec<String>, InvalidCuration> {
    let columns = cells(header)?;
    if columns[0] != "iri" {
        return Err(InvalidCuration::FirstColumnNotIri { found: columns[0].clone() });
    }
    for (index, column) in columns.iter().enumerate().skip(1) {
        if !is_comment(column) && !is_value_column(column) {
            return Err(InvalidCuration::MalformedColumn { column: column.clone() });
        }
        if columns[..index].contains(column) {
            return Err(InvalidCuration::DuplicateColumn { column: column.clone() });
        }
    }
    Ok(columns)
}

fn is_comment(column: &str) -> bool {
    column.starts_with('#')
}

fn is_value_column(column: &str) -> bool {
    let (key, lang) = key_and_lang(column);
    is_curation_name(key) && lang.is_none_or(is_curation_name)
}

fn key_and_lang(column: &str) -> (&str, Option<&str>) {
    match column.split_once('@') {
        Some((key, lang)) => (key, Some(lang)),
        None => (column, None),
    }
}

/// Appends the values of the row on `line` to `values`, and records the row's IRI in `rows` with
/// its line.
fn read_row(
    line: usize,
    row: &str,
    columns: &[String],
    known: &BTreeSet<&str>,
    rows: &mut BTreeMap<String, usize>,
    values: &mut Vec<CuratedValue>,
) -> Result<(), InvalidCuration> {
    let mut cells = cells(row)?;
    if row.is_empty() {
        return Err(InvalidCuration::BlankLine);
    }
    if cells.len() != columns.len() {
        return Err(InvalidCuration::WrongCellCount { expected: columns.len(), found: cells.len() });
    }
    let iri = std::mem::take(&mut cells[0]);
    if !known.contains(iri.as_str()) {
        return Err(InvalidCuration::UnknownResource { iri });
    }
    if let Some(&first_line) = rows.get(&iri) {
        return Err(InvalidCuration::DuplicateRow { iri, first_line });
    }
    for (column, text) in columns.iter().zip(cells).skip(1) {
        if is_comment(column) || text.is_empty() {
            continue;
        }
        if text.starts_with(is_padding) || text.ends_with(is_padding) {
            return Err(InvalidCuration::PaddedValue { column: column.clone() });
        }
        let (key, lang) = key_and_lang(column);
        values.push(CuratedValue {
            resource: ResourceIri(iri.clone()),
            key: key.to_string(),
            lang: lang.map(str::to_string),
            text,
        });
    }
    rows.insert(iri, line);
    Ok(())
}

fn is_padding(character: char) -> bool {
    character.is_whitespace() || matches!(character, '\u{200b}' | '\u{feff}')
}

/// Where the reader stands in a line.
enum Position {
    CellStart,
    Bare,
    Quoted,
    /// After a quote inside a quoted cell: the cell's end, or the first half of a doubled quote.
    QuoteInQuoted,
}

/// The values of a line's cells; never empty, since a line without a comma is one cell.
fn cells(line: &str) -> Result<Vec<String>, InvalidCuration> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut position = Position::CellStart;
    for character in line.chars() {
        if character.is_control() || matches!(character, '\u{2028}' | '\u{2029}') {
            return Err(InvalidCuration::ControlCharacter { character });
        }
        position = match (position, character) {
            (Position::CellStart, '"') => Position::Quoted,
            (Position::Quoted, '"') => Position::QuoteInQuoted,
            (Position::QuoteInQuoted, '"') | (Position::Quoted, _) => {
                cell.push(character);
                Position::Quoted
            }
            (Position::CellStart | Position::Bare | Position::QuoteInQuoted, ',') => {
                cells.push(std::mem::take(&mut cell));
                Position::CellStart
            }
            (Position::Bare, '"') | (Position::QuoteInQuoted, _) => return Err(InvalidCuration::StrayQuote),
            (Position::CellStart | Position::Bare, _) => {
                cell.push(character);
                Position::Bare
            }
        };
    }
    if matches!(position, Position::Quoted) {
        return Err(InvalidCuration::UnterminatedQuote);
    }
    cells.push(cell);
    Ok(cells)
}
