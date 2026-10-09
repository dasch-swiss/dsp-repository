//! A project's curation file, read into the port's curated values.
//!
//! The file is UTF-8 text, one record per line, each line ended by a line feed; the last may lack
//! it. Line 1 is the header: `iri`, then one column per curated key, named `key` or `key@lang`
//! with both parts curation names (`contract::is_curation_name`), or a comment column, whose name
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

use cpe_ports::{contract, CuratedValue, ResourceIri};

use crate::{CurationFault, InvalidCuration};

/// The curated values of a curation file's bytes, sorted, or the first fault met. `known` holds
/// the IRIs of the served resources: a row for any other IRI is a fault. Pure: no I/O.
///
/// Faults are met in this order: bytes that are not UTF-8, wherever they stand; then line by line.
/// Within a line, a refused character or a misplaced quote comes first, from left to right. The
/// header's columns then follow from left to right. A row is then blank, of the wrong cell count,
/// for an unknown IRI or the IRI's second, in that order, and only then holds a padded value, the
/// leftmost first.
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
    contract::is_curation_name(key) && lang.is_none_or(contract::is_curation_name)
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
