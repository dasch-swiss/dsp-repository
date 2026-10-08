//! A resource's plain data ARK, computed from its DSP IRI.
//!
//! [`data_ark`] must stay equal to dsp-api's `resourceIriToArkUrl`, and [`check_digit`] to its
//! `Base64UrlCheckDigit`. The prefix lives in `cpe_ports::DATA_ARK_PREFIX` and is also spelled in
//! the incubator's `translate.py` and `engine/src/ir.rs`; a change goes to all of them.

use cpe_ports::{DataArk, DATA_ARK_PREFIX};

/// The base64url alphabet without padding, the check digit's character values in order.
const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

const RESOURCE_IRI_PREFIX: &str = "http://rdfh.ch/";

/// Why an IRI has no data ARK.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArkError {
    /// Not `http://rdfh.ch/<shortcode>/<id>` with a `[0-9A-Fa-f]{4}` shortcode and no further
    /// path segment.
    #[error("not a DSP resource IRI")]
    NotResourceIri,
    /// A character of the id outside base64url; the shortcode is checked separately.
    #[error("a character outside the base64url alphabet: {0:?}")]
    InvalidChar(char),
    /// The id is empty.
    #[error("an empty resource id")]
    Empty,
    /// Every character weighs 0 (an all-`A` id).
    #[error("a resource id whose check digit sum is zero")]
    ZeroSum,
}

/// The check digit of a base64url id: position `i` (0-based) of an id of length `n` weighs
/// `n + 1 - i`, and the digit is the character whose value is `(64 - total % 64) % 64`.
pub(crate) fn check_digit(id: &str) -> Result<char, ArkError> {
    let n = id.chars().count();
    if n == 0 {
        return Err(ArkError::Empty);
    }
    let mut total: usize = 0;
    for (i, c) in id.chars().enumerate() {
        let value = ALPHABET.find(c).ok_or(ArkError::InvalidChar(c))?;
        total += value * (n + 1 - i);
    }
    if total == 0 {
        return Err(ArkError::ZeroSum);
    }
    let modulus = ALPHABET.len();
    Ok(char::from(ALPHABET.as_bytes()[(modulus - total % modulus) % modulus]))
}

/// The plain data ARK of `http://rdfh.ch/<shortcode>/<id>`: the shortcode upper-cased, the id
/// followed by its check digit, every `-` (the digit included) escaped as `=`.
pub(crate) fn data_ark(iri: &str) -> Result<DataArk, ArkError> {
    let rest = iri.strip_prefix(RESOURCE_IRI_PREFIX).ok_or(ArkError::NotResourceIri)?;
    let (shortcode, id) = rest.split_once('/').ok_or(ArkError::NotResourceIri)?;
    if shortcode.len() != 4 || !shortcode.chars().all(|c| c.is_ascii_hexdigit()) || id.contains('/') {
        return Err(ArkError::NotResourceIri);
    }
    let digit = check_digit(id)?;
    let escaped = format!("{id}{digit}").replace('-', "=");
    Ok(DataArk(format!(
        "{DATA_ARK_PREFIX}{}/{escaped}",
        shortcode.to_ascii_uppercase()
    )))
}
