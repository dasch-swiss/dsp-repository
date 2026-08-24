//! Generating the one-time code and the opaque tokens, and comparing a
//! submitted code without leaking it through timing.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::{Rng, RngCore};
use subtle::ConstantTimeEq;

/// How many wrong entries one code tolerates before it is dead (REQ-6.4).
///
/// Per **code**. The counter that survives a resend is on the account — see
/// [`super::AuthConfig::max_failed`] — because REQ-6.4's three strikes on their
/// own hand out a fresh budget every time a new code is issued.
pub(crate) const MAX_CODE_ATTEMPTS: u32 = 3;

/// A six-digit code, uniform over `000000..=999999`.
///
/// `random_range` samples the range by rejection, so every value is equally
/// likely. `rng % 1_000_000` would not be: unless the generator's range is an
/// exact multiple of the modulus, the low values come up slightly more often,
/// which is the classic modulo bias and hands an attacker a better-than-uniform
/// guessing order.
///
/// Six digits is ≈19.93 bits, marginally under OWASP ASVS 6.5.4's 20-bit floor.
/// That is accepted rather than overlooked: the code lives ten minutes, tolerates
/// three wrong entries, and sits behind a per-account counter and a per-IP limit.
/// Seven digits would clear the floor and cost every user an extra keystroke on
/// a control they use daily.
pub(crate) fn code() -> String {
    let value: u32 = rand::rng().random_range(0..1_000_000);
    // Padded, so a value below 100000 is still six digits — dropping the leading
    // zeros would shrink the space by a tenth for a tenth of the codes.
    format!("{value:06}")
}

/// An opaque 256-bit token: the session id, and the pre-auth browser binding.
///
/// URL-safe base64 without padding, so it needs no escaping in a cookie value.
/// 256 bits rather than a UUID's 122 because these are bearer credentials looked
/// up directly, and the cost of the extra bytes is nothing.
pub(crate) fn token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Whether a submitted code equals the stored one, in constant time.
///
/// A `==` on strings returns at the first differing byte, so the time it takes
/// reveals how long a shared prefix is — enough to recover a six-digit code one
/// digit at a time. `subtle` compares every byte regardless.
///
/// Length is not secret here (a code is always six digits), and `ConstantTimeEq`
/// on slices reports unequal lengths as a mismatch without comparing.
pub(crate) fn code_matches(submitted: &str, stored: &str) -> bool {
    submitted.as_bytes().ct_eq(stored.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_a_code_is_always_six_digits() {
        for _ in 0..1_000 {
            let code = code();
            assert_eq!(code.len(), 6, "{code}");
            assert!(code.chars().all(|c| c.is_ascii_digit()), "{code}");
        }
    }

    #[test]
    fn test_the_whole_six_digit_range_is_reachable() {
        // Both ends: a generator that dropped leading zeros would never produce a
        // value below 100000, and one that sampled `0..999999` would never
        // produce the top value. Over 20,000 draws either gap is a certainty to
        // catch — the chance of seeing no value below 100000 is 0.9^20000.
        let mut lowest = u32::MAX;
        let mut highest = 0;
        for _ in 0..20_000 {
            let value: u32 = code().parse().unwrap();
            lowest = lowest.min(value);
            highest = highest.max(value);
        }
        assert!(
            lowest < 100_000,
            "no code with a leading zero in 20,000 draws: lowest was {lowest}"
        );
        assert!(
            highest >= 900_000,
            "no code in the top tenth in 20,000 draws: highest was {highest}"
        );
    }

    #[test]
    fn test_codes_do_not_repeat_in_a_short_run() {
        // Not a distribution test — a smoke test that the generator is seeded and
        // advancing rather than returning one value. With a million values, 500
        // draws collide about 12% of the time, so this asserts on the count of
        // distinct values rather than on all of them being distinct.
        let distinct: HashSet<String> = (0..500).map(|_| code()).collect();
        assert!(
            distinct.len() > 400,
            "500 draws produced only {} distinct codes",
            distinct.len()
        );
    }

    #[test]
    fn test_a_token_is_long_url_safe_and_unique() {
        let distinct: HashSet<String> = (0..1_000).map(|_| token()).collect();
        assert_eq!(distinct.len(), 1_000, "256-bit tokens must not collide");
        for token in distinct.iter().take(20) {
            // 32 bytes in base64 without padding.
            assert_eq!(token.len(), 43, "{token}");
            assert!(
                token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "a cookie value must need no escaping: {token}"
            );
        }
    }

    #[test]
    fn test_code_matches_only_on_an_exact_match() {
        assert!(code_matches("123456", "123456"));
        assert!(!code_matches("123457", "123456"));
        assert!(!code_matches("023456", "123456"));
    }

    #[test]
    fn test_a_prefix_or_a_longer_string_is_not_a_match() {
        // A length-tolerant comparison would accept a submitted prefix, which
        // turns a six-digit search into a one-digit one.
        assert!(!code_matches("12345", "123456"));
        assert!(!code_matches("1234567", "123456"));
        assert!(!code_matches("", "123456"));
        assert!(!code_matches("123456", ""));
    }
}
