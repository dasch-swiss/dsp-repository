//! The `pages` reader's page-picker grouping (book-page plan step 7,
//! Choice F): a pure, total function from a compound's members to the
//! picker's groups. No rendering here — `compound_viewer` draws the groups.
//!
//! Two modes (`ir::PickerMode`):
//!
//! - **Numbered:** runs of [`RUN_LENGTH`] pages by position.
//! - **Signatures:** the book's own collation, read from each page label's leading signature token
//!   (`a1r` → gathering `a`, leaf 1, recto):
//!   1. a label is a signature when its leading token is 1–3 of `[A-Za-z&?]`, digits and `r`/`v`,
//!      followed by the end or one of whitespace `,` `;` `(`. `&` and `?` are gathering marks too
//!      (plan decision 11);
//!   2. the gathering key is case-sensitive (`A` ≠ `a`);
//!   3. a group is a contiguous run of one key; a key recurring after another starts a new group;
//!   4. unparsed labels before the first signature are the front flyleaves (Vorsatz), after the
//!      last the back flyleaves (Nachsatz); an unparsed label between two pages of one gathering
//!      takes the slot the sequence implies (after `r` the same leaf's `v`, after `v` the next
//!      leaf's `r`) and draws as a dash; any other unparsed run is a flat "Weitere" group;
//!   5. pages on one (leaf, side) share that cell, in `seq` order;
//!   6. a book where fewer than half the labels parse falls back to numbered runs.
//!
//! Invariant (unit- and data-tested): every member appears in exactly one
//! entry; groups follow `seq` order, and so do the entries of each cell.

use cpe_core::ir::{MemberHeader, PickerMode};

/// Pages per numbered run (the design's value; no attribute until a second
/// project needs another).
pub(crate) const RUN_LENGTH: usize = 20;

/// A flat group whose every label is at most this many characters draws as
/// the dense number grid; otherwise as the wider flyleaf grid.
const NUMBER_GRID_MAX_CHARS: usize = 6;

/// How one entry draws; its accessible name is always the full label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Display {
    /// The label, verbatim.
    Label,
    /// A dash: an empty label, or an unparsed one in an inferred slot.
    Dash,
    /// A signature under its gathering, without the repeated key (`1r`).
    SideOnly(String),
}

/// One page in the picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// 0-based position in the compound's members.
    pub(crate) member_index: usize,
    /// The member's label, verbatim.
    pub(crate) label: String,
    pub(crate) display: Display,
}

impl Entry {
    /// The text the entry shows, before any repeat mark.
    pub(crate) fn visible(&self) -> &str {
        match &self.display {
            Display::Label => &self.label,
            Display::SideOnly(side) => side,
            Display::Dash => "—",
        }
    }
}

/// For each entry of one cell, which occurrence of its visible text it is
/// (1-based): four pages all showing "5v" get 1, 2, 3, 4.
pub(crate) fn repeat_ordinals(cell: &[Entry]) -> Vec<usize> {
    cell.iter()
        .enumerate()
        .map(|(i, e)| 1 + cell[..i].iter().filter(|p| p.visible() == e.visible()).count())
        .collect()
}

/// The visible mark for a repeat: superscript digits (`²`, `¹⁰`), empty for
/// the first occurrence.
pub(crate) fn ordinal_mark(n: usize) -> String {
    if n < 2 {
        return String::new();
    }
    n.to_string()
        .chars()
        .map(|d| match d {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            other => other,
        })
        .collect()
}

/// One leaf of a gathering: its recto cell over its verso cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Leaf {
    pub(crate) leaf: u32,
    pub(crate) recto: Vec<Entry>,
    pub(crate) verso: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GroupKind {
    /// Front flyleaves.
    Vorsatz,
    /// Back flyleaves.
    Nachsatz,
    /// A signature run, e.g. key `b`.
    Gathering { key: String },
    /// An unparsed run between two gatherings.
    Weitere,
    /// A numbered run, 1-based positions `first..=last`.
    Range { first: usize, last: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GroupBody {
    /// A gathering's leaves, in the order they first appear.
    Leaves(Vec<Leaf>),
    /// Every other group: entries in `seq` order.
    Flat(Vec<Entry>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PickerGroup {
    pub(crate) kind: GroupKind,
    pub(crate) body: GroupBody,
}

impl PickerGroup {
    /// Every entry, in drawing order (a gathering: leaf by leaf, recto cell
    /// then verso cell).
    pub(crate) fn entries(&self) -> Vec<&Entry> {
        match &self.body {
            GroupBody::Leaves(leaves) => leaves.iter().flat_map(|l| l.recto.iter().chain(l.verso.iter())).collect(),
            GroupBody::Flat(entries) => entries.iter().collect(),
        }
    }

    /// Whether this flat group draws as the dense number grid: every label
    /// is at most [`NUMBER_GRID_MAX_CHARS`] characters. Gatherings never do.
    pub(crate) fn number_grid(&self) -> bool {
        match &self.body {
            GroupBody::Leaves(_) => false,
            GroupBody::Flat(entries) => entries.iter().all(|e| e.label.chars().count() <= NUMBER_GRID_MAX_CHARS),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Recto,
    Verso,
}

/// A parsed leading signature token. `side_only` is the token without its
/// key (`1r`), sliced from the label so it reads exactly as written.
struct Signature<'a> {
    key: &'a str,
    leaf: u32,
    side: Side,
    side_only: &'a str,
}

/// Parse a label's leading signature token (Choice F rule 1), or `None`.
fn parse_signature(label: &str) -> Option<Signature<'_>> {
    let key_len = label
        .bytes()
        .take_while(|b| b.is_ascii_alphabetic() || *b == b'&' || *b == b'?')
        .count();
    if key_len == 0 || key_len > 3 {
        return None;
    }
    let rest = &label[key_len..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let side = match rest.as_bytes().get(digits) {
        Some(b'r') => Side::Recto,
        Some(b'v') => Side::Verso,
        _ => return None,
    };
    let after = rest.as_bytes().get(digits + 1);
    if !matches!(after, None | Some(b',' | b';' | b'(')) && !after.is_some_and(u8::is_ascii_whitespace) {
        return None;
    }
    Some(Signature {
        key: &label[..key_len],
        leaf: rest[..digits].parse().ok()?,
        side,
        side_only: &rest[..=digits],
    })
}

/// Group a compound's members for the picker (see the module doc).
pub(crate) fn group(members: &[MemberHeader], mode: PickerMode) -> Vec<PickerGroup> {
    match mode {
        PickerMode::Numbered => numbered(members),
        PickerMode::Signatures => signatures(members).unwrap_or_else(|| numbered(members)),
    }
}

/// A flat entry: the label verbatim, or a dash when it is empty.
fn flat_entry(member_index: usize, m: &MemberHeader) -> Entry {
    let display = if m.name.trim().is_empty() {
        Display::Dash
    } else {
        Display::Label
    };
    Entry { member_index, label: m.name.clone(), display }
}

fn numbered(members: &[MemberHeader]) -> Vec<PickerGroup> {
    members
        .chunks(RUN_LENGTH)
        .enumerate()
        .map(|(run, chunk)| {
            let start = run * RUN_LENGTH;
            PickerGroup {
                kind: GroupKind::Range { first: start + 1, last: start + chunk.len() },
                body: GroupBody::Flat(chunk.iter().enumerate().map(|(i, m)| flat_entry(start + i, m)).collect()),
            }
        })
        .collect()
}

/// The gathering being built: its key, leaves, and the last slot filled.
struct Run {
    key: String,
    leaves: Vec<Leaf>,
    last: (u32, Side),
}

impl Run {
    fn place(&mut self, leaf: u32, side: Side, entry: Entry) {
        let at = match self.leaves.iter().position(|l| l.leaf == leaf) {
            Some(at) => at,
            None => {
                self.leaves.push(Leaf { leaf, recto: Vec::new(), verso: Vec::new() });
                self.leaves.len() - 1
            }
        };
        let cell = match side {
            Side::Recto => &mut self.leaves[at].recto,
            Side::Verso => &mut self.leaves[at].verso,
        };
        cell.push(entry);
        self.last = (leaf, side);
    }

    fn into_group(self) -> PickerGroup {
        PickerGroup {
            kind: GroupKind::Gathering { key: self.key },
            body: GroupBody::Leaves(self.leaves),
        }
    }
}

/// Signatures mode, or `None` when fewer than half the labels parse (F6).
fn signatures(members: &[MemberHeader]) -> Option<Vec<PickerGroup>> {
    let sigs: Vec<Option<Signature<'_>>> = members.iter().map(|m| parse_signature(&m.name)).collect();
    let parsed = sigs.iter().filter(|s| s.is_some()).count();
    if parsed * 2 < members.len() {
        return None;
    }
    let (Some(first), Some(last)) = (sigs.iter().position(Option::is_some), sigs.iter().rposition(Option::is_some))
    else {
        // No members at all.
        return Some(Vec::new());
    };

    let mut groups = Vec::new();
    let flat = |range: std::ops::Range<usize>| -> Vec<Entry> { range.map(|i| flat_entry(i, &members[i])).collect() };
    if first > 0 {
        groups.push(PickerGroup {
            kind: GroupKind::Vorsatz,
            body: GroupBody::Flat(flat(0..first)),
        });
    }

    let mut run: Option<Run> = None;
    let mut weitere: Vec<Entry> = Vec::new();
    for i in first..=last {
        let label = members[i].name.clone();
        if let Some(sig) = &sigs[i] {
            // A pending "Weitere" run only ever sits between two different
            // keys, so a new key closes the gathering, then the run.
            if run.as_ref().is_none_or(|r| r.key != sig.key) {
                if let Some(done) = run.take() {
                    groups.push(done.into_group());
                }
                if !weitere.is_empty() {
                    groups.push(PickerGroup {
                        kind: GroupKind::Weitere,
                        body: GroupBody::Flat(std::mem::take(&mut weitere)),
                    });
                }
                run = Some(Run {
                    key: sig.key.to_owned(),
                    leaves: Vec::new(),
                    last: (sig.leaf, sig.side),
                });
            }
            let entry = Entry {
                member_index: i,
                label,
                display: Display::SideOnly(sig.side_only.to_owned()),
            };
            if let Some(r) = run.as_mut() {
                r.place(sig.leaf, sig.side, entry);
            }
            continue;
        }
        // Unparsed, strictly inside `first..last`, so a later signature
        // exists. Between two pages of the current gathering it takes the
        // implied slot; otherwise it joins a flat "Weitere" run.
        let next_key = sigs[i + 1..=last].iter().flatten().next().map(|s| s.key);
        match run.as_mut() {
            Some(r) if weitere.is_empty() && next_key == Some(r.key.as_str()) => {
                let (leaf, side) = match r.last {
                    (leaf, Side::Recto) => (leaf, Side::Verso),
                    (leaf, Side::Verso) => (leaf.saturating_add(1), Side::Recto),
                };
                r.place(leaf, side, Entry { member_index: i, label, display: Display::Dash });
            }
            _ => weitere.push(flat_entry(i, &members[i])),
        }
    }
    if let Some(done) = run.take() {
        groups.push(done.into_group());
    }
    if last + 1 < members.len() {
        groups.push(PickerGroup {
            kind: GroupKind::Nachsatz,
            body: GroupBody::Flat(flat(last + 1..members.len())),
        });
    }
    Some(groups)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members(labels: &[&str]) -> Vec<MemberHeader> {
        labels
            .iter()
            .enumerate()
            .map(|(i, l)| MemberHeader {
                id: format!("m{i}"),
                class: "Page".to_owned(),
                name: (*l).to_owned(),
            })
            .collect()
    }

    fn kinds(groups: &[PickerGroup]) -> Vec<&GroupKind> {
        groups.iter().map(|g| &g.kind).collect()
    }

    fn gathering(key: &str) -> GroupKind {
        GroupKind::Gathering { key: key.to_owned() }
    }

    fn leaves(g: &PickerGroup) -> &[Leaf] {
        match &g.body {
            GroupBody::Leaves(l) => l,
            GroupBody::Flat(_) => panic!("expected a gathering: {g:?}"),
        }
    }

    fn indices(cell: &[Entry]) -> Vec<usize> {
        cell.iter().map(|e| e.member_index).collect()
    }

    /// Asserts the module invariant on `groups` over `n` members.
    fn assert_invariant(groups: &[PickerGroup], n: usize, what: &str) {
        let mut seen: Vec<usize> = groups
            .iter()
            .flat_map(|g| g.entries().into_iter().map(|e| e.member_index))
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..n).collect::<Vec<_>>(), "{what}: every member exactly once");
        let firsts: Vec<usize> = groups
            .iter()
            .map(|g| g.entries().iter().map(|e| e.member_index).min().expect("non-empty group"))
            .collect();
        assert!(firsts.windows(2).all(|w| w[0] < w[1]), "{what}: groups in seq order");
        for g in groups {
            let cells: Vec<Vec<usize>> = match &g.body {
                GroupBody::Leaves(ls) => ls.iter().flat_map(|l| [indices(&l.recto), indices(&l.verso)]).collect(),
                GroupBody::Flat(es) => vec![indices(es)],
            };
            for c in cells {
                assert!(c.windows(2).all(|w| w[0] < w[1]), "{what}: cell in seq order: {c:?}");
            }
        }
    }

    #[test]
    fn a_leading_signature_token_parses_with_trailing_annotation() {
        for label in [
            "a1r, Titelblatt, recto",
            "a1r; Titelblatt recto",
            r#"A2r (falsch nummeriert: "A iij")"#,
            "b1v und b2r",
            "D5v (doppelt gescannt???)",
            "a1r",
            "&8v",
            "?1r",
        ] {
            assert!(parse_signature(label).is_some(), "{label} parses");
        }
        for label in [
            "Vorsatzblatt, recto",
            "(1)r",
            "1r",
            "1 recto",
            "abcd1r",
            "a1rx",
            "[missing]",
            "158",
            "",
        ] {
            assert!(parse_signature(label).is_none(), "{label} does not parse");
        }
        let sig = parse_signature("b1v und b2r").expect("parses");
        assert_eq!((sig.key, sig.leaf, sig.side, sig.side_only), ("b", 1, Side::Verso, "1v"));
    }

    #[test]
    fn upper_and_lower_case_keys_are_different_gatherings() {
        let g = group(&members(&["z8v", "A1r", "A1v"]), PickerMode::Signatures);
        assert_eq!(kinds(&g), [&gathering("z"), &gathering("A")]);
    }

    #[test]
    fn a_key_recurring_after_another_starts_a_new_group() {
        let g = group(&members(&["a1r", "b1r", "a2r"]), PickerMode::Signatures);
        assert_eq!(kinds(&g), [&gathering("a"), &gathering("b"), &gathering("a")]);
    }

    #[test]
    fn unparsed_labels_before_and_after_are_flyleaves() {
        let g = group(
            &members(&[
                "Einband, Vorderdeckel, aussen",
                "Vorsatzblatt, recto",
                "a1r",
                "a1v",
                "a2r",
                "a2v",
                "Nachsatzblatt, verso",
            ]),
            PickerMode::Signatures,
        );
        assert_eq!(kinds(&g), [&GroupKind::Vorsatz, &gathering("a"), &GroupKind::Nachsatz]);
        assert_eq!(g[0].entries()[1].display, Display::Label);
        assert_eq!(g[1].entries()[0].display, Display::SideOnly("1r".to_owned()));
    }

    #[test]
    fn an_unparsed_label_after_a_recto_takes_the_verso() {
        let g = group(&members(&["a1r", "158", "a2r"]), PickerMode::Signatures);
        let ls = leaves(&g[0]);
        assert_eq!(indices(&ls[0].verso), [1]);
        assert_eq!(ls[0].verso[0].display, Display::Dash);
        assert_eq!(ls[0].verso[0].label, "158");
    }

    #[test]
    fn an_unparsed_label_after_a_verso_takes_the_next_recto() {
        let g = group(&members(&["a1r", "a1v", "[missing]", "a2v"]), PickerMode::Signatures);
        let ls = leaves(&g[0]);
        assert_eq!((ls[1].leaf, indices(&ls[1].recto)), (2, vec![2]));
        assert_eq!(indices(&ls[1].verso), [3]);
    }

    #[test]
    fn an_unparsed_run_between_two_gatherings_is_weitere() {
        let g = group(&members(&["a1r", "a1v", "Testbild", "b1r", "b1v"]), PickerMode::Signatures);
        assert_eq!(kinds(&g), [&gathering("a"), &GroupKind::Weitere, &gathering("b")]);
    }

    #[test]
    fn duplicate_scans_share_one_cell_in_seq_order() {
        let g = group(
            &members(&[
                "D5r",
                "D5v (doppelt gescannt???)",
                "D5v (doppelt gescannt???)",
                "D5v (doppelt gescannt???)",
                "D6r",
            ]),
            PickerMode::Signatures,
        );
        let ls = leaves(&g[0]);
        assert_eq!(indices(&ls[0].verso), [1, 2, 3]);
        assert_invariant(&g, 5, "D5v triple");
    }

    #[test]
    fn repeats_in_one_cell_count_by_visible_text() {
        let g = group(
            &members(&["D5r", "D5v", "D5v (doppelt)", "D5v (doppelt)", "D5v", "D6r"]),
            PickerMode::Signatures,
        );
        let cell = &leaves(&g[0])[0].verso;
        assert_eq!(repeat_ordinals(cell), [1, 2, 3, 4]);
        assert_eq!(repeat_ordinals(&leaves(&g[0])[0].recto), [1]);
        let marks: Vec<String> = [1, 2, 3, 4, 9, 10, 12].into_iter().map(ordinal_mark).collect();
        assert_eq!(marks, ["", "²", "³", "⁴", "⁹", "¹⁰", "¹²"]);
    }

    #[test]
    fn a_dash_beside_a_signature_is_not_a_repeat() {
        // "158" after D5r takes the D5v slot as a dash; the real D5v follows.
        let g = group(&members(&["D5r", "158", "D5v", "D6r"]), PickerMode::Signatures);
        let cell = &leaves(&g[0])[0].verso;
        assert_eq!(repeat_ordinals(cell), [1, 1]);
    }

    #[test]
    fn et_and_placeholder_marks_are_gatherings() {
        let g = group(&members(&["z8v", "&1r", "&8v", "?1r", "?8v", "A1r"]), PickerMode::Signatures);
        assert_eq!(kinds(&g), [&gathering("z"), &gathering("&"), &gathering("?"), &gathering("A")]);
    }

    #[test]
    fn a_mostly_unparsed_book_falls_back_to_numbered_runs() {
        let labels: Vec<String> = (1..=45).map(|n| format!("{n}r")).collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let g = group(&members(&labels), PickerMode::Signatures);
        assert_eq!(
            kinds(&g),
            [
                &GroupKind::Range { first: 1, last: 20 },
                &GroupKind::Range { first: 21, last: 40 },
                &GroupKind::Range { first: 41, last: 45 },
            ]
        );
    }

    #[test]
    fn one_stray_signature_in_a_numbered_book_stays_numbered() {
        let mut labels: Vec<String> = (1..=30).map(|n| format!("{n} recto")).collect();
        labels[3] = "a1r".to_owned();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let g = group(&members(&labels), PickerMode::Signatures);
        assert_eq!(
            kinds(&g),
            [
                &GroupKind::Range { first: 1, last: 20 },
                &GroupKind::Range { first: 21, last: 30 }
            ]
        );
    }

    #[test]
    fn short_labels_draw_the_number_grid_and_long_ones_do_not() {
        let short = group(&members(&["1", "2", "123456"]), PickerMode::Numbered);
        assert!(short[0].number_grid());
        let long = group(&members(&["1 recto", "1 verso"]), PickerMode::Numbered);
        assert!(!long[0].number_grid());
    }

    #[test]
    fn a_single_member_book_is_one_group() {
        let g = group(&members(&["a1r"]), PickerMode::Signatures);
        assert_eq!(kinds(&g), [&gathering("a")]);
        let g = group(&members(&["Titelblatt"]), PickerMode::Signatures);
        assert_eq!(kinds(&g), [&GroupKind::Range { first: 1, last: 1 }]);
        assert!(group(&[], PickerMode::Signatures).is_empty());
    }

    // ── the real data: every Incunabula book ──────────────────────────────

    /// Every Incunabula book's members, in `compound_parts.seq` order, read from
    /// `testdata/incunabula-book-members.tsv`: one line per member, with the book's id, the
    /// member's id and the member's name, separated by tabs. The file is an extract of the
    /// project's data in dsp-incubator. A name is read as it stands, so no field is trimmed. A
    /// book's lines are adjacent and in `seq` order: a book whose lines are apart is read as two.
    fn incunabula_books() -> Vec<(String, Vec<MemberHeader>)> {
        let extract = include_str!("testdata/incunabula-book-members.tsv");
        let mut books: Vec<(String, Vec<MemberHeader>)> = Vec::new();
        for line in extract.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            let [book, id, name] = fields[..] else {
                panic!("a line of the extract has three fields: {line:?}");
            };
            let member = MemberHeader {
                id: id.to_owned(),
                class: "Page".to_owned(),
                name: name.to_owned(),
            };
            match books.last_mut() {
                Some((b, ms)) if b == book => ms.push(member),
                _ => books.push((book.to_owned(), vec![member])),
            }
        }
        books
    }

    #[test]
    fn every_page_of_every_incunabula_book_appears_exactly_once() {
        let books = incunabula_books();
        assert_eq!(books.len(), 19, "all 19 books");
        for (book, ms) in &books {
            let g = group(ms, PickerMode::Signatures);
            assert_invariant(&g, ms.len(), book);
        }
        let by = |name: &str| {
            let (_, ms) = books.iter().find(|(b, _)| b == name).expect("book present");
            group(ms, PickerMode::Signatures)
        };
        let melusine = by("melusine");
        assert_eq!(melusine.len(), 10);
        assert!(melusine.iter().all(|g| matches!(g.kind, GroupKind::Range { .. })));
        let quadragesimale = by("quadragesimale");
        assert!(kinds(&quadragesimale).contains(&&gathering("&")));
        assert!(kinds(&quadragesimale).contains(&&gathering("?")));
        let bereitung = by("bereitung");
        assert_eq!(
            bereitung
                .iter()
                .filter(|g| matches!(g.kind, GroupKind::Gathering { .. }))
                .count(),
            32
        );
    }
}
