//! Whole-page bodies. Each returns the `<main>` content; the server wraps it in
//! `deposit_shell::view::page`.

pub mod approved;
pub mod projects;
pub mod review;
pub mod section;
pub mod states;

/// RDU's tables: a white surface with a small capitals header row on Neutral 50.
pub(crate) const LIST_TABLE: &str = "bg-white border border-neutral-200 [&_th]:bg-neutral-50 [&_th]:border-b \
                                     [&_th]:border-neutral-200 [&_th]:text-xs [&_th]:uppercase \
                                     [&_th]:tracking-wider [&_th]:text-neutral-600 [&_td]:border-neutral-100";
