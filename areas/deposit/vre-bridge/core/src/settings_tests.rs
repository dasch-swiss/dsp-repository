use std::time::Duration;

use chrono::NaiveTime;

use crate::{EntryRef, RawSettings, ServerId, Settings, SettingsError};

/// Stands in for a signing key; the `=` padding is what a real base64 key ends with.
const KEY: &str = "MC4CAQAwBQYDK2VwBCIEIFAKEKEYMATERIALFAKEKEYMATERIAL0000000000==";
/// What no error and no `Debug` may contain.
const KEY_PART: &str = "FAKEKEYMATERIAL";

fn raw() -> RawSettings {
    RawSettings {
        sync_dir: Some("/srv/stand-in/sync".to_owned()),
        media_dir: Some("/srv/stand-in/media".to_owned()),
        lock_file: Some("/var/lib/deposit/vre-bridge.lock".to_owned()),
        allowlist: Some("0803=prod".to_owned()),
        servers: Some("prod=https://api.dasch.swiss,stage=https://api.stage.dasch.swiss".to_owned()),
        key_ids: Some("prod=dsp-repository-prod-2026-11".to_owned()),
        signing_keys: Some(format!("prod={KEY}").into()),
        ..RawSettings::default()
    }
}

fn parse(raw: RawSettings) -> Settings {
    Settings::parse(raw).unwrap().expect("output locations are set")
}

fn error(raw: RawSettings) -> SettingsError {
    let error = Settings::parse(raw).unwrap_err();
    assert!(!error.to_string().contains(KEY_PART), "{error}");
    assert!(!format!("{error:?}").contains(KEY_PART), "{error:?}");
    error
}

fn number(text: &str) -> Option<String> {
    Some(text.to_owned())
}

fn server(id: &str) -> ServerId {
    ServerId::parse(id).unwrap()
}

#[test]
fn test_parse_no_output_location_inactive() {
    assert_eq!(Settings::parse(RawSettings::default()), Ok(None));
    let blank = RawSettings {
        sync_dir: Some(String::new()),
        media_dir: Some("  ".to_owned()),
        ..raw()
    };
    assert_eq!(Settings::parse(blank), Ok(None));
}

#[test]
fn test_parse_one_output_location_or_no_lock_file_names_missing_variable() {
    let cases = [
        (RawSettings { media_dir: None, ..raw() }, "EDITOR_VRE_BRIDGE_MEDIA_DIR"),
        (RawSettings { sync_dir: None, ..raw() }, "EDITOR_VRE_BRIDGE_SYNC_DIR"),
        (RawSettings { lock_file: None, ..raw() }, "EDITOR_VRE_BRIDGE_LOCK_FILE"),
    ];
    for (raw, var) in cases {
        assert_eq!(error(raw), SettingsError::Missing { var });
    }
}

#[test]
fn test_parse_minimal_settings_take_defaults() {
    let settings = parse(raw());

    assert_eq!(settings.service_name, "dsp-repository");
    assert_eq!(settings.inputs_dir.to_str(), Some("areas/deposit/vre-bridge/inputs"));
    assert_eq!(settings.work_dir, std::env::temp_dir());
    assert_eq!(settings.nightly_at, NaiveTime::from_hms_opt(2, 0, 0).unwrap());
    assert_eq!(settings.time_zone, "Europe/Zurich");
    assert_eq!(settings.dump_retries, 3);
    assert_eq!(settings.dump_retry, Duration::from_secs(1200));
    assert_eq!(settings.request_timeout, Duration::from_secs(120));
    assert_eq!(settings.export_timeout, Duration::from_secs(3600));
    assert_eq!(settings.export_poll, Duration::from_secs(15));
    assert_eq!(settings.download_timeout, Duration::from_secs(3600));
    assert_eq!(settings.file_timeout, Duration::from_secs(300));
    assert_eq!(settings.max_dump_bytes, 256 << 20);
    assert_eq!(settings.max_unpacked_bytes, 512 << 20);
    assert_eq!(settings.max_file_bytes, 512 << 20);
}

#[test]
fn test_parse_every_variable_set_kept() {
    let settings = parse(RawSettings {
        iiif_bases: Some("prod=https://iiif.dasch.swiss".to_owned()),
        service_name: Some("dsp-repository-prod".to_owned()),
        inputs_dir: Some("/app/vre-bridge/inputs".to_owned()),
        work_dir: Some("/var/lib/vre-bridge-work".to_owned()),
        nightly_at: Some("03:30".to_owned()),
        time_zone: Some("UTC".to_owned()),
        dump_retries: number("0"),
        dump_retry_secs: number("60"),
        request_timeout_secs: number("1"),
        export_timeout_secs: number("2"),
        export_poll_secs: number(" 3 "),
        download_timeout_secs: number("4"),
        file_timeout_secs: number("5"),
        max_dump_bytes: number("6"),
        max_unpacked_bytes: number("7"),
        max_file_bytes: number("8"),
        ..raw()
    });

    let prod = &settings.servers[&server("prod")];
    assert_eq!(prod.base_url, "https://api.dasch.swiss");
    assert_eq!(prod.iiif_base.as_deref(), Some("https://iiif.dasch.swiss"));
    assert_eq!(prod.key_id, "dsp-repository-prod-2026-11");
    assert_eq!(prod.signing_key.expose(), KEY);
    assert_eq!(settings.service_name, "dsp-repository-prod");
    assert_eq!(settings.work_dir.to_str(), Some("/var/lib/vre-bridge-work"));
    assert_eq!(settings.nightly_at, NaiveTime::from_hms_opt(3, 30, 0).unwrap());
    assert_eq!(settings.time_zone, "UTC");
    assert_eq!(settings.dump_retries, 0);
    assert_eq!(settings.dump_retry, Duration::from_secs(60));
    assert_eq!(
        [
            settings.request_timeout,
            settings.export_timeout,
            settings.export_poll,
            settings.download_timeout,
            settings.file_timeout
        ],
        [1, 2, 3, 4, 5].map(Duration::from_secs)
    );
    assert_eq!(
        [
            settings.max_dump_bytes,
            settings.max_unpacked_bytes,
            settings.max_file_bytes
        ],
        [6, 7, 8]
    );
}

#[test]
fn test_parse_list_grammar_trims_drops_empty_entries_and_sorts() {
    let settings = parse(RawSettings {
        allowlist: Some(" , 083C = prod ,,0803=prod, ".to_owned()),
        ..raw()
    });

    let entries: Vec<_> = settings
        .allowlist
        .entries()
        .iter()
        .map(|e| (e.shortcode.as_str(), e.server.as_str()))
        .collect();
    assert_eq!(entries, [("0803", "prod"), ("083C", "prod")]);
}

#[test]
fn test_parse_only_servers_the_allowlist_uses_kept() {
    let settings = parse(raw());

    assert_eq!(settings.servers.keys().collect::<Vec<_>>(), [&server("prod")]);
}

#[test]
fn test_parse_empty_allowlist_accepted() {
    let settings = parse(RawSettings { allowlist: None, ..raw() });

    assert!(settings.allowlist.entries().is_empty());
    assert!(settings.servers.is_empty());
}

#[test]
fn test_parse_entry_without_equals_or_with_empty_part_named_by_position() {
    let cases = [
        (
            "0803=prod,0804prod",
            SettingsError::NoEquals { var: "EDITOR_VRE_BRIDGE_ALLOWLIST", position: 2 },
        ),
        (
            "0803=",
            SettingsError::Empty { var: "EDITOR_VRE_BRIDGE_ALLOWLIST", position: 1 },
        ),
        (
            "=prod",
            SettingsError::Empty { var: "EDITOR_VRE_BRIDGE_ALLOWLIST", position: 1 },
        ),
    ];
    for (allowlist, expected) in cases {
        assert_eq!(error(RawSettings { allowlist: Some(allowlist.to_owned()), ..raw() }), expected);
    }
}

#[test]
fn test_parse_duplicate_key_refused() {
    let allowlist = RawSettings { allowlist: Some("0803=prod,0803=stage".to_owned()), ..raw() };
    assert_eq!(
        error(allowlist),
        SettingsError::Duplicate {
            var: "EDITOR_VRE_BRIDGE_ALLOWLIST",
            entry: EntryRef::Key("0803".to_owned())
        }
    );

    let servers = RawSettings {
        servers: Some("prod=https://a.example,prod=https://b.example".to_owned()),
        ..raw()
    };
    assert_eq!(
        error(servers),
        SettingsError::Duplicate {
            var: "EDITOR_VRE_BRIDGE_SERVERS",
            entry: EntryRef::Key("prod".to_owned())
        }
    );
}

#[test]
fn test_parse_duplicate_signing_key_named_by_position_only() {
    let keys = RawSettings {
        signing_keys: Some(format!("prod={KEY},prod={KEY}").into()),
        ..raw()
    };

    assert_eq!(
        error(keys),
        SettingsError::Duplicate {
            var: "EDITOR_VRE_BRIDGE_SIGNING_KEYS",
            entry: EntryRef::Position(2)
        }
    );
}

#[test]
fn test_parse_signing_key_without_server_id_never_quoted() {
    // Split on its first `=`, the bare key reads as an id that is most of the key.
    let bare = RawSettings { signing_keys: Some(KEY.to_owned().into()), ..raw() };
    assert_eq!(
        error(bare),
        SettingsError::ServerId {
            var: "EDITOR_VRE_BRIDGE_SIGNING_KEYS",
            entry: EntryRef::Position(1)
        }
    );

    let no_equals = RawSettings {
        signing_keys: Some("MC4CAQAwBQYDK2VwBCIEIFAKEKEYMATERIAL".to_owned().into()),
        ..raw()
    };
    assert_eq!(
        error(no_equals),
        SettingsError::NoEquals { var: "EDITOR_VRE_BRIDGE_SIGNING_KEYS", position: 1 }
    );
}

#[test]
fn test_parse_lower_case_shortcode_refused() {
    let lower = RawSettings { allowlist: Some("083c=prod".to_owned()), ..raw() };

    assert_eq!(
        error(lower),
        SettingsError::Shortcode {
            var: "EDITOR_VRE_BRIDGE_ALLOWLIST",
            entry: EntryRef::Key("083c".to_owned())
        }
    );
}

#[test]
fn test_parse_malformed_server_id_refused() {
    let upper = RawSettings { allowlist: Some("0803=Prod".to_owned()), ..raw() };

    assert_eq!(
        error(upper),
        SettingsError::ServerId {
            var: "EDITOR_VRE_BRIDGE_ALLOWLIST",
            entry: EntryRef::Key("0803".to_owned())
        }
    );
}

#[test]
fn test_parse_unknown_server_refused() {
    let allowlisted = RawSettings { allowlist: Some("0803=test".to_owned()), ..raw() };
    assert_eq!(
        error(allowlisted),
        SettingsError::NoEntry { var: "EDITOR_VRE_BRIDGE_SERVERS", server: server("test") }
    );

    let key_id = RawSettings { key_ids: Some("prod=a,test=b".to_owned()), ..raw() };
    assert_eq!(
        error(key_id),
        SettingsError::UnknownServer {
            var: "EDITOR_VRE_BRIDGE_KEY_IDS",
            entry: EntryRef::Key("test".to_owned())
        }
    );

    let key = RawSettings {
        signing_keys: Some(format!("prod={KEY},test={KEY}").into()),
        ..raw()
    };
    assert_eq!(
        error(key),
        SettingsError::UnknownServer {
            var: "EDITOR_VRE_BRIDGE_SIGNING_KEYS",
            entry: EntryRef::Position(2)
        }
    );
}

#[test]
fn test_parse_allowlisted_server_without_key_id_or_signing_key_refused() {
    let no_key = RawSettings { signing_keys: None, ..raw() };
    assert_eq!(
        error(no_key),
        SettingsError::NoEntry {
            var: "EDITOR_VRE_BRIDGE_SIGNING_KEYS",
            server: server("prod")
        }
    );

    let no_kid = RawSettings { key_ids: Some("stage=kid".to_owned()), ..raw() };
    assert_eq!(
        error(no_kid),
        SettingsError::NoEntry { var: "EDITOR_VRE_BRIDGE_KEY_IDS", server: server("prod") }
    );
}

#[test]
fn test_parse_url_with_trailing_slash_other_scheme_or_credentials_refused() {
    for url in [
        "https://api.dasch.swiss/",
        "ftp://api.dasch.swiss",
        "http://api.dasch.swiss",
        "http://localhost.dasch.swiss",
        "api.dasch.swiss",
        "https://",
        "https:///v2",
        "https://user:pw@api.dasch.swiss",
        "https://api.dasch.swiss?token=x",
        "https://api.dasch.swiss#x",
        "https://api.dasch.swiss /x",
    ] {
        let servers = RawSettings { servers: Some(format!("prod={url}")), ..raw() };
        assert_eq!(
            error(servers),
            SettingsError::Url {
                var: "EDITOR_VRE_BRIDGE_SERVERS",
                entry: EntryRef::Key("prod".to_owned())
            }
        );
    }
    let iiif = RawSettings {
        iiif_bases: Some("prod=https://iiif.dasch.swiss/".to_owned()),
        ..raw()
    };
    assert_eq!(
        error(iiif),
        SettingsError::Url {
            var: "EDITOR_VRE_BRIDGE_IIIF_BASES",
            entry: EntryRef::Key("prod".to_owned())
        }
    );
}

#[test]
fn test_parse_http_url_to_a_loopback_host_accepted() {
    for url in [
        "http://127.0.0.1:8080",
        "http://localhost:8080/api",
        "http://[::1]:8080",
    ] {
        let settings = parse(RawSettings { servers: Some(format!("prod={url}")), ..raw() });
        assert_eq!(settings.servers[&server("prod")].base_url, url);
    }
}

#[test]
fn test_parse_unparsable_nightly_at_refused() {
    for value in ["2:00", "24:00", "02:60", "02:00:00", "noon"] {
        let raw = RawSettings { nightly_at: Some(value.to_owned()), ..raw() };
        assert_eq!(error(raw), SettingsError::Time { var: "EDITOR_VRE_BRIDGE_NIGHTLY_AT" });
    }
}

#[test]
fn test_parse_key_pasted_into_another_variable_never_quoted() {
    let cases = [
        RawSettings { allowlist: Some(format!("{KEY}prod")), ..raw() },
        RawSettings { allowlist: Some(format!("{KEY}=prod")), ..raw() },
        RawSettings {
            servers: Some(format!("{KEY}=https://api.dasch.swiss")),
            ..raw()
        },
        RawSettings { key_ids: Some(format!("prod=a,{KEY}=b")), ..raw() },
        RawSettings { iiif_bases: Some(format!("prod={KEY}")), ..raw() },
        RawSettings { nightly_at: Some(KEY.to_owned()), ..raw() },
    ];
    for raw in cases {
        // `error` asserts that neither Display nor Debug holds the key.
        error(raw);
    }
}

#[test]
fn test_parse_zero_time_limit_or_size_cap_refused() {
    let zero_timeout = RawSettings { export_poll_secs: number("0"), ..raw() };
    assert_eq!(
        error(zero_timeout),
        SettingsError::Zero { var: "EDITOR_VRE_BRIDGE_EXPORT_POLL_SECS" }
    );

    let zero_cap = RawSettings { max_unpacked_bytes: number("0"), ..raw() };
    assert_eq!(
        error(zero_cap),
        SettingsError::Zero { var: "EDITOR_VRE_BRIDGE_MAX_UNPACKED_BYTES" }
    );
}

#[test]
fn test_parse_number_not_a_whole_number_refused_unquoted() {
    for text in ["three", "-1", "1.5", "18446744073709551616"] {
        let raw = RawSettings { dump_retries: number(text), ..raw() };
        let error = error(raw);
        assert_eq!(error, SettingsError::Number { var: "EDITOR_VRE_BRIDGE_DUMP_RETRIES" });
        assert!(!error.to_string().contains(text), "{error}");
    }
    let key = RawSettings { max_dump_bytes: number(KEY), ..raw() };
    assert_eq!(error(key), SettingsError::Number { var: "EDITOR_VRE_BRIDGE_MAX_DUMP_BYTES" });
}

#[test]
fn test_parse_blank_number_takes_the_default() {
    let settings = parse(RawSettings { dump_retry_secs: number("  "), ..raw() });
    assert_eq!(settings.dump_retry, Duration::from_secs(1200));
}

#[test]
fn test_debug_raw_settings_and_settings_contain_no_key() {
    let raw = raw();
    assert!(!format!("{raw:?}").contains(KEY_PART));

    let settings = parse(raw);
    assert!(!format!("{settings:?}").contains(KEY_PART));
    assert!(!format!("{settings:#?}").contains(KEY_PART));
}
