use std::path::Path;
use std::{fs, io};

use tempfile::TempDir;

use crate::{known_projects, read_token, Generation, InvalidToken, Provenance, ReadyToken, TokenError, TOKEN_FILE};

const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// The canonical encoding of [`token`], byte for byte.
const ENCODED: &str = concat!(
    r#"{"token_version":1,"generation":"20261120T010203Z-9f3a1c2e","#,
    r#""previous_generation":"20261119T010155Z-0b1d2e3f","as_of":"2026-11-20T01:02:03.456Z","#,
    r#""server":"prod","interim_format_version":1,"transformation_version":1,"#,
    r#""inputs_digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}"#,
    "\n",
);

fn generation(name: &str) -> Generation {
    Generation::parse(name).expect("a well-formed generation")
}

fn token() -> ReadyToken {
    ReadyToken {
        provenance: Provenance {
            generation: generation("20261120T010203Z-9f3a1c2e"),
            as_of: "2026-11-20T01:02:03.456Z".to_owned(),
            server: "prod".to_owned(),
            interim_format_version: 1,
            transformation_version: 1,
            inputs_digest: DIGEST.to_owned(),
        },
        previous_generation: Some(generation("20261119T010155Z-0b1d2e3f")),
    }
}

/// [`ENCODED`] with `from` replaced by `to`; panics if `from` does not occur.
fn encoded_with(from: &str, to: &str) -> Vec<u8> {
    assert!(ENCODED.contains(from), "{from}");
    ENCODED.replace(from, to).into_bytes()
}

fn assert_invalid(bytes: &[u8], expected: &InvalidToken) {
    let result = ReadyToken::parse(bytes);
    assert_eq!(result, Err(TokenError::Invalid(expected.clone())));
}

/// A root holding `<root>/<shortcode>/` with the canonical token in it.
fn project_with_token(root: &Path, shortcode: &str) {
    fs::create_dir(root.join(shortcode)).expect("create the project directory");
    fs::write(root.join(shortcode).join(TOKEN_FILE), ENCODED).expect("write the token");
}

fn assert_unreadable(result: Result<Option<ReadyToken>, TokenError>) {
    assert!(matches!(result, Err(TokenError::Unreadable(_))), "{result:?}");
}

#[test]
fn test_generation_parse_well_formed_name_accepted() {
    assert_eq!(generation("20261120T010203Z-9f3a1c2e").as_str(), "20261120T010203Z-9f3a1c2e");
}

#[test]
fn test_generation_parse_malformed_names_refused() {
    for name in [
        "2026-11-20",
        "",
        "20261120T010203Z-9F3A1C2E",
        "20261120T010203Z-9f3a1c2",
        "20261120T010203Z-9f3a1c2e0",
        "20261120t010203z-9f3a1c2e",
        "2026112xT010203Z-9f3a1c2e",
        "../20261120T010203Z-9f3a",
    ] {
        assert_eq!(Generation::parse(name), None, "{name}");
    }
}

#[test]
fn test_token_to_bytes_canonical_encoding_byte_for_byte() {
    assert_eq!(String::from_utf8(token().to_bytes()).expect("UTF-8"), ENCODED);
}

#[test]
fn test_token_parse_canonical_encoding_round_trips() {
    let parsed = ReadyToken::parse(ENCODED.as_bytes()).expect("the canonical token parses");
    assert_eq!(parsed, token());
    assert_eq!(parsed.to_bytes(), ENCODED.as_bytes());
}

#[test]
fn test_token_parse_null_previous_generation_round_trips() {
    let bytes = encoded_with(r#""20261119T010155Z-0b1d2e3f""#, "null");
    let parsed = ReadyToken::parse(&bytes).expect("a first generation's token parses");
    assert_eq!(parsed.previous_generation, None);
    assert_eq!(parsed.to_bytes(), bytes);
}

#[test]
fn test_token_parse_unknown_field_ignored() {
    let bytes = encoded_with(r#""server":"prod","#, r#""server":"prod","later":[1,2],"#);
    assert_eq!(ReadyToken::parse(&bytes), Ok(token()));
}

#[test]
fn test_token_parse_another_token_version_refused() {
    assert_invalid(
        &encoded_with(r#""token_version":1"#, r#""token_version":2"#),
        &InvalidToken::TokenVersion(2),
    );
}

#[test]
fn test_token_parse_another_version_without_version_one_fields_refused_as_version() {
    assert_invalid(br#"{"token_version":2,"pointer":"elsewhere"}"#, &InvalidToken::TokenVersion(2));
}

#[test]
fn test_token_parse_bad_generation_refused() {
    assert_invalid(
        &encoded_with(r#""generation":"20261120T010203Z-9f3a1c2e""#, r#""generation":"2026-11-20""#),
        &InvalidToken::Generation,
    );
}

#[test]
fn test_token_parse_bad_previous_generation_refused() {
    assert_invalid(
        &encoded_with("20261119T010155Z-0b1d2e3f", "2026-11-19"),
        &InvalidToken::Generation,
    );
}

#[test]
fn test_token_parse_bad_server_ids_refused() {
    for server in ["", "Prod", "prod/1", "a234567890123456789012345678901234"] {
        let bytes = encoded_with(r#""server":"prod""#, &format!(r#""server":"{server}""#));
        assert_invalid(&bytes, &InvalidToken::Server);
    }
}

#[test]
fn test_token_parse_upper_case_digest_refused() {
    assert_invalid(
        &encoded_with("0123456789abcdef0123", "0123456789ABCDEF0123"),
        &InvalidToken::InputsDigest,
    );
}

#[test]
fn test_token_parse_digest_without_prefix_or_wrong_length_refused() {
    for digest in [
        &DIGEST["sha256:".len()..],
        &DIGEST[..DIGEST.len() - 1],
        "md5:0123456789abcdef",
    ] {
        let bytes = encoded_with(DIGEST, digest);
        assert_invalid(&bytes, &InvalidToken::InputsDigest);
    }
}

#[test]
fn test_token_parse_not_a_token_refused_as_malformed() {
    for bytes in [&b"not json"[..], b"", b"[1]", b"{\"token_version\":1}", b"\xff"] {
        let result = ReadyToken::parse(bytes);
        assert!(
            matches!(result, Err(TokenError::Invalid(InvalidToken::Malformed(_)))),
            "{result:?}"
        );
    }
}

#[test]
fn test_read_token_valid_token_returned() {
    let root = TempDir::new().expect("temp dir");
    project_with_token(root.path(), "0803");
    assert_eq!(read_token(root.path(), "0803"), Ok(Some(token())));
}

#[test]
fn test_read_token_refused_shortcode_is_none() {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir(root.path().join("x")).expect("create x");
    fs::write(root.path().join("x").join(TOKEN_FILE), ENCODED).expect("write a token beside the root");
    let inner = root.path().join("inner");
    fs::create_dir(&inner).expect("create the root");
    for shortcode in ["../x", "083c", "0803/..", ""] {
        assert_eq!(read_token(&inner, shortcode), Ok(None), "{shortcode}");
    }
}

#[test]
fn test_read_token_project_without_directory_is_none() {
    let root = TempDir::new().expect("temp dir");
    assert_eq!(read_token(root.path(), "0803"), Ok(None));
}

#[test]
fn test_read_token_project_without_token_is_none() {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir(root.path().join("0803")).expect("create the project directory");
    assert_eq!(read_token(root.path(), "0803"), Ok(None));
}

#[test]
fn test_read_token_directory_named_token_unreadable() {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir_all(root.path().join("0803").join(TOKEN_FILE)).expect("create a directory named .ready");
    assert_unreadable(read_token(root.path(), "0803"));
}

#[test]
fn test_read_token_project_path_a_file_unreadable() {
    let root = TempDir::new().expect("temp dir");
    fs::write(root.path().join("0803"), ENCODED).expect("write a file where the project directory belongs");
    assert_unreadable(read_token(root.path(), "0803"));
}

#[cfg(unix)]
#[test]
fn test_read_token_symlinked_token_unreadable() {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir(root.path().join("0803")).expect("create the project directory");
    fs::write(root.path().join("elsewhere"), ENCODED).expect("write the link target");
    std::os::unix::fs::symlink(root.path().join("elsewhere"), root.path().join("0803").join(TOKEN_FILE))
        .expect("link the token");
    assert_unreadable(read_token(root.path(), "0803"));
}

#[cfg(unix)]
#[test]
fn test_read_token_symlinked_project_directory_unreadable() {
    let root = TempDir::new().expect("temp dir");
    project_with_token(root.path(), "0804");
    std::os::unix::fs::symlink(root.path().join("0804"), root.path().join("0803")).expect("link the project");
    assert_unreadable(read_token(root.path(), "0803"));
}

#[test]
fn test_read_token_malformed_token_invalid() {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir(root.path().join("0803")).expect("create the project directory");
    fs::write(root.path().join("0803").join(TOKEN_FILE), "{\"token_version\":").expect("write the token");
    let result = read_token(root.path(), "0803");
    assert!(
        matches!(result, Err(TokenError::Invalid(InvalidToken::Malformed(_)))),
        "{result:?}"
    );
}

#[test]
fn test_generation_dir_joins_root_shortcode_and_generation() {
    let generation = generation("20261120T010203Z-9f3a1c2e");
    assert_eq!(
        crate::generation_dir(Path::new("/srv/sync"), "0803", &generation),
        Some(Path::new("/srv/sync/0803/20261120T010203Z-9f3a1c2e").to_path_buf())
    );
    assert_eq!(crate::generation_dir(Path::new("/srv/sync"), "../x", &generation), None);
}

#[test]
fn test_known_projects_lists_only_token_holding_shortcode_directories() {
    let root = TempDir::new().expect("temp dir");
    let root = root.path();
    project_with_token(root, "0804");
    project_with_token(root, "0803");
    project_with_token(root, "083c");
    fs::create_dir(root.join("lost+found")).expect("create lost+found");
    fs::write(root.join("PROVENANCE"), "notes").expect("write PROVENANCE");
    fs::create_dir(root.join("0805")).expect("create a directory without a token");
    #[cfg(unix)]
    {
        project_with_token(root, "target");
        std::os::unix::fs::symlink(root.join("target"), root.join("0806")).expect("link a project");
    }
    assert_eq!(known_projects(root).expect("list the root"), ["0803", "0804"]);
}

#[test]
fn test_known_projects_missing_root_is_an_error() {
    let root = TempDir::new().expect("temp dir");
    let result = known_projects(&root.path().join("absent"));
    assert_eq!(result.map_err(|e| e.kind()), Err(io::ErrorKind::NotFound));
}

#[test]
fn test_token_parse_missing_previous_generation_refused_as_malformed() {
    let bytes = encoded_with(r#""previous_generation":"20261119T010155Z-0b1d2e3f","#, "");
    let result = ReadyToken::parse(&bytes);
    assert!(
        matches!(result, Err(TokenError::Invalid(InvalidToken::Malformed(_)))),
        "{result:?}"
    );
}

#[test]
fn test_read_token_oversized_token_invalid() {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir(root.path().join("0803")).expect("create the project directory");
    // A valid token with trailing whitespace past the cap: truncated, it would still parse.
    let padding = " ".repeat(usize::try_from(crate::TOKEN_MAX_BYTES).expect("fits"));
    fs::write(root.path().join("0803").join(TOKEN_FILE), format!("{ENCODED}{padding}")).expect("write the token");
    let result = read_token(root.path(), "0803");
    assert!(
        matches!(result, Err(TokenError::Invalid(InvalidToken::Malformed(_)))),
        "{result:?}"
    );
}

#[cfg(unix)]
#[test]
fn test_known_projects_symlinked_token_skipped() {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir(root.path().join("0803")).expect("create the project directory");
    fs::write(root.path().join("elsewhere"), ENCODED).expect("write the link target");
    std::os::unix::fs::symlink(root.path().join("elsewhere"), root.path().join("0803").join(TOKEN_FILE))
        .expect("link the token");
    assert_eq!(known_projects(root.path()).expect("list the root"), Vec::<String>::new());
}
