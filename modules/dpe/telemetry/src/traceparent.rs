/// Validate a W3C traceparent string.
/// Format: `00-{32 lowercase hex}-{16 lowercase hex}-{2 lowercase hex}` (55 chars).
/// Rejects all-zero trace-id and span-id per W3C Trace Context spec.
/// Requires lowercase hex per W3C (uppercase is non-conformant).
pub fn is_valid_traceparent(tp: &str) -> bool {
    let bytes = tp.as_bytes();
    bytes.len() == 55
        && bytes[2] == b'-'
        && bytes[35] == b'-'
        && bytes[52] == b'-'
        && tp.starts_with("00-")
        && tp
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f') || b == b'-')
        && &tp[3..35] != "00000000000000000000000000000000"
        && &tp[36..52] != "0000000000000000"
}

/// Validate and return a traceparent string reference.
/// Returns None if the value is absent or invalid.
pub fn validated_traceparent(tp: &Option<String>) -> Option<&str> {
    tp.as_deref().filter(|s| is_valid_traceparent(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_traceparent() {
        assert!(is_valid_traceparent(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        ));
        assert!(is_valid_traceparent(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00"
        ));
    }

    #[test]
    fn rejects_zero_trace_id() {
        assert!(!is_valid_traceparent(
            "00-00000000000000000000000000000000-00f067aa0ba902b7-01"
        ));
    }

    #[test]
    fn rejects_zero_span_id() {
        assert!(!is_valid_traceparent(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01"
        ));
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(!is_valid_traceparent("00-abc-def-01"));
        assert!(!is_valid_traceparent(""));
    }

    #[test]
    fn rejects_uppercase_hex() {
        assert!(!is_valid_traceparent(
            "00-4BF92F3577B34DA6A3CE929D0E0E4736-00F067AA0BA902B7-01"
        ));
    }

    #[test]
    fn rejects_non_hex_characters() {
        assert!(!is_valid_traceparent(
            "00-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz-00f067aa0ba902b7-01"
        ));
    }

    #[test]
    fn rejects_wrong_version() {
        assert!(!is_valid_traceparent(
            "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        ));
    }

    #[test]
    fn validated_returns_some_for_valid() {
        let tp = Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string());
        assert!(validated_traceparent(&tp).is_some());
    }

    #[test]
    fn validated_returns_none_for_invalid() {
        let tp = Some("invalid".to_string());
        assert!(validated_traceparent(&tp).is_none());
    }

    #[test]
    fn validated_returns_none_for_none() {
        assert!(validated_traceparent(&None).is_none());
    }

    // --- Property-based tests ---

    mod properties {
        use super::*;
        use proptest::prelude::*;

        fn valid_traceparent_strategy() -> impl Strategy<Value = String> {
            ("[0-9a-f]{32}", "[0-9a-f]{16}", "[0-9a-f]{2}")
                .prop_filter("non-zero trace and span IDs", |(trace, span, _)| {
                    trace != "00000000000000000000000000000000"
                        && span != "0000000000000000"
                })
                .prop_map(|(trace, span, flags)| format!("00-{trace}-{span}-{flags}"))
        }

        proptest! {
            #[test]
            fn valid_traceparent_always_accepted(tp in valid_traceparent_strategy()) {
                prop_assert!(is_valid_traceparent(&tp), "should accept: {tp}");
            }

            #[test]
            fn arbitrary_string_never_panics(s in "\\PC{0,100}") {
                let _ = is_valid_traceparent(&s);
            }

            #[test]
            fn accepted_strings_match_w3c_format(s in "\\PC{0,100}") {
                if is_valid_traceparent(&s) {
                    prop_assert_eq!(s.len(), 55);
                    prop_assert!(s.starts_with("00-"));
                }
            }

            #[test]
            fn validated_agrees_with_is_valid(s in "\\PC{0,100}") {
                let opt = Some(s.clone());
                let result = validated_traceparent(&opt);
                prop_assert_eq!(result.is_some(), is_valid_traceparent(&s));
            }
        }
    }
}
