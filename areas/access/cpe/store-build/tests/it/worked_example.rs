//! The worked example of `docs/src/cpe/config-language.md`'s "`remodel` block" section is
//! Incunabula's committed block: the two lower to the same mapping. The document and the project
//! are this repository's and the lowering is the vendored `cpe-core`'s, so the comparison is made
//! here.

use cpe_core::validate::load_remodel;

const CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../projects/incunabula/config.kdl"));

/// The first fenced `kdl` block of `docs/src/cpe/config-language.md`'s "`remodel` block" section:
/// the worked example, which opens with the minimal `site` block that `load_remodel` asks for.
fn worked_example() -> &'static str {
    const DOC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../docs/src/cpe/config-language.md"
    ));
    let section = DOC
        .split_once("\n## `remodel` block\n")
        .expect("the document has the section")
        .1;
    let fence = section.split_once("\n```kdl\n").expect("the section has a fenced kdl block").1;
    fence.split_once("\n```").expect("the fenced block ends").0
}

#[test]
fn the_worked_example_of_the_config_language_lowers_to_incunabulas_block() {
    let example = load_remodel(worked_example()).unwrap_or_else(|e| panic!("the worked example lowers: {e:?}"));
    let committed = load_remodel(CONFIG).unwrap_or_else(|e| panic!("the block lowers: {e:?}"));

    // A block that is not empty, so that two empty blocks cannot be what is equal.
    assert_eq!(committed.classes.len(), 3);
    assert_eq!(
        example, committed,
        "docs/src/cpe/config-language.md's worked example is not what projects/incunabula/config.kdl \
         lowers to: change the document and the block together"
    );
}

/// The control of the test above: the comparison fails for a one-word change of the example.
#[test]
fn the_worked_example_differs_from_incunabulas_block_when_one_word_changes() {
    let changed = worked_example().replacen("CC BY 4.0", "CC BY 4.1", 1);
    assert_ne!(changed, worked_example(), "the word to change is in the example");

    let example = load_remodel(&changed).expect("the changed example lowers");
    let committed = load_remodel(CONFIG).expect("the block lowers");

    assert_ne!(example, committed);
}
