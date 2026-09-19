//! JSON renderer — newline-delimited JSON output.
//!
//! Every response is a single object with `_meta` first, plus exactly one of
//! `data` (success) or `error` (failure):
//!
//! ```json
//! {"_meta": {"server": "…", "auth": "…", "exit_code": 0}, "data": { … }}
//! {"_meta": {"server": "…", "auth": "…", "exit_code": 3}, "error": {"kind": "…", "message": "…"}}
//! ```
//!
//! `data` is an object for single-result commands and an array for list
//! commands (Phase 4+). The shape is uniform across every command so a
//! consumer always parses one object from stdout: if `.error` is present it
//! failed, otherwise read `.data`. The `server` lives only in `_meta` — it is
//! not repeated inside `data`. Key order is deterministic (serde_json
//! `preserve_order`); `_meta` is always first. See dsp-cli/ADR-0003 and dsp-cli/ADR-0012.

use std::io::{self, Write};

use serde_json::json;

use crate::diagnostic::Diagnostic;
use crate::model::{
    DataModelDetail, DataModelStructure, DatePoint, ProjectDetail, ResourceDetail, ValueContent, VocabularyDetail,
};
use crate::render::auth::{AuthLoginOutcome, AuthLogoutOutcome, AuthSetTokenOutcome, AuthStatusOutcome};
use crate::render::dump::{DumpDeleteOutcome, DumpOutcome};
use crate::render::vocabulary::{NestedVocabularyNode, nest_vocabulary_detail};
use crate::render::{
    DataModelListView, MetaContext, ProjectListView, Renderer, ResourceListPagination, ResourceListView,
    ResourceTypeListView, VocabularyListView,
};

/// Renders output as newline-delimited JSON.
pub struct JsonRenderer {
    out: Box<dyn Write>,
}

impl JsonRenderer {
    /// Creates a renderer writing to stdout.
    pub fn new() -> Self {
        Self { out: Box::new(io::stdout()) }
    }

    /// Creates a renderer writing to an arbitrary `Write` sink (used in tests).
    pub fn with_writer(w: impl Write + 'static) -> Self {
        Self { out: Box::new(w) }
    }
}

impl Default for JsonRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the `_meta` block common to every JSON output object.
///
/// `server`/`auth` are included only when their source string is non-empty;
/// `exit_code` is always included. On every existing success-path call site,
/// `server_label`/`auth_state` are always non-empty (set by `Config::resolve`
/// and `read_auth_state` respectively), so this omission only fires for the
/// top-level error path (plan 032 D3), which has no server/auth context yet.
fn meta_block(meta: &MetaContext, exit_code: u8) -> serde_json::Value {
    use serde_json::Map;
    let mut m = Map::new();
    if !meta.server_label.is_empty() {
        m.insert("server".into(), serde_json::Value::String(meta.server_label.clone()));
    }
    if !meta.auth_state.is_empty() {
        m.insert("auth".into(), serde_json::Value::String(meta.auth_state.clone()));
    }
    m.insert("exit_code".into(), serde_json::Value::from(exit_code));
    serde_json::Value::Object(m)
}

/// Build a verbatim, lossless `[{"value": …, "language": …}]` array from a
/// `LocalizedText` slice (plan 034). json is the lossless path for vocabulary
/// labels/comments — no per-language column collapsing (that is tabular-only,
/// see `src/render/vocabulary.rs`). `language: None` serialises as JSON
/// `null`, matching `project_describe`'s `ProjectDescription` array precedent.
fn localized_text_array(items: &[crate::model::LocalizedText]) -> Vec<serde_json::Value> {
    items
        .iter()
        .map(|l| {
            json!({
                "value": l.value,
                "language": l.language,
            })
        })
        .collect()
}

/// Build a JSON value object for a single `ValueContent` (per dsp-cli/ADR-0013 matrix).
///
/// Key order is deterministic: `value_type` is always first, then type-specific
/// keys in the order specified by the matrix. Raw server values are kept verbatim
/// (no sanitisation — dsp-cli/ADR-0003 fidelity; sanitisation is prose-only per D7).
fn value_content_to_json(vc: &ValueContent) -> serde_json::Value {
    use serde_json::Map;
    let mut m = Map::new();
    match vc {
        ValueContent::Text(s) => {
            m.insert("value_type".into(), serde_json::Value::String("text".into()));
            m.insert("text".into(), serde_json::Value::String(s.clone()));
        }
        ValueContent::Integer(n) => {
            m.insert("value_type".into(), serde_json::Value::String("integer".into()));
            m.insert("value".into(), serde_json::Value::Number((*n).into()));
        }
        ValueContent::Decimal(s) => {
            m.insert("value_type".into(), serde_json::Value::String("decimal".into()));
            m.insert("value".into(), serde_json::Value::String(s.clone()));
        }
        ValueContent::Boolean(b) => {
            m.insert("value_type".into(), serde_json::Value::String("boolean".into()));
            m.insert("value".into(), serde_json::Value::Bool(*b));
        }
        ValueContent::Date(dv) => {
            m.insert("value_type".into(), serde_json::Value::String("date".into()));
            m.insert("calendar".into(), serde_json::Value::String(dv.calendar.clone()));
            // Build start/end point objects — omit absent sub-fields (null for era when None).
            let point_to_json = |p: &DatePoint| {
                json!({
                    "year": p.year,
                    "month": p.month,
                    "day": p.day,
                    "era": p.era,
                })
            };
            m.insert("start".into(), point_to_json(&dv.start));
            m.insert("end".into(), point_to_json(&dv.end));
        }
        ValueContent::Time(s) => {
            m.insert("value_type".into(), serde_json::Value::String("time".into()));
            m.insert("value".into(), serde_json::Value::String(s.clone()));
        }
        ValueContent::Uri(s) => {
            m.insert("value_type".into(), serde_json::Value::String("uri".into()));
            m.insert("value".into(), serde_json::Value::String(s.clone()));
        }
        ValueContent::Color(s) => {
            m.insert("value_type".into(), serde_json::Value::String("color".into()));
            m.insert("value".into(), serde_json::Value::String(s.clone()));
        }
        ValueContent::Geoname(s) => {
            m.insert("value_type".into(), serde_json::Value::String("geoname".into()));
            m.insert("value".into(), serde_json::Value::String(s.clone()));
        }
        ValueContent::VocabularyItem { node_iri, label } => {
            m.insert("value_type".into(), serde_json::Value::String("vocabulary-item".into()));
            m.insert("node_iri".into(), serde_json::Value::String(node_iri.clone()));
            let label_val = match label {
                Some(s) => serde_json::Value::String(s.clone()),
                None => serde_json::Value::Null,
            };
            m.insert("label".into(), label_val);
        }
        ValueContent::Link { target_iri, target_label } => {
            m.insert("value_type".into(), serde_json::Value::String("link".into()));
            m.insert("target_iri".into(), serde_json::Value::String(target_iri.clone()));
            let tl_val = match target_label {
                Some(s) => serde_json::Value::String(s.clone()),
                None => serde_json::Value::Null,
            };
            m.insert("target_label".into(), tl_val);
        }
        ValueContent::File(fv) => {
            use crate::model::resource_type::ValueType;
            let type_token = fv.value_type.as_token().to_string();
            m.insert("value_type".into(), serde_json::Value::String(type_token));
            m.insert("filename".into(), serde_json::Value::String(fv.filename.clone()));
            m.insert("url".into(), serde_json::Value::String(fv.url.clone()));
            // width/height: only meaningful for still-image, null for others.
            match fv.value_type {
                ValueType::StillImage => {
                    let w_val: serde_json::Value = fv
                        .width
                        .map_or(serde_json::Value::Null, |w| serde_json::Value::Number(w.into()));
                    let h_val: serde_json::Value = fv
                        .height
                        .map_or(serde_json::Value::Null, |h| serde_json::Value::Number(h.into()));
                    m.insert("width".into(), w_val);
                    m.insert("height".into(), h_val);
                }
                _ => {
                    m.insert("width".into(), serde_json::Value::Null);
                    m.insert("height".into(), serde_json::Value::Null);
                }
            }
        }
        ValueContent::Raw { value_type, text } => {
            m.insert("value_type".into(), serde_json::Value::String(value_type.clone()));
            m.insert("text".into(), serde_json::Value::String(text.clone()));
        }
    }
    serde_json::Value::Object(m)
}

/// Map a `Diagnostic` variant to its stable JSON `kind` string (per dsp-cli/ADR-0012).
fn diagnostic_kind(diag: &Diagnostic) -> &'static str {
    match diag {
        Diagnostic::Usage(_) => "usage",
        Diagnostic::AuthRequired(_) => "auth_required",
        Diagnostic::NotFound(_) => "not_found",
        Diagnostic::ServerError(_) => "server_error",
        Diagnostic::Network(_) => "network",
        Diagnostic::Conflict(_) => "conflict",
        Diagnostic::Io(_) => "io",
        Diagnostic::Internal(_) | Diagnostic::NotImplemented(_) => "internal",
    }
}

impl Renderer for JsonRenderer {
    fn diagnostic(&mut self, diag: &Diagnostic, meta: &MetaContext) -> Result<(), Diagnostic> {
        // JSON errors emit the full dsp-cli/ADR-0012 error envelope to stdout so a JSON
        // consumer has a single stream to parse (not stdout + stderr).
        let exit_code = diag.exit_category() as u8;
        let obj = json!({
            "_meta": meta_block(meta, exit_code),
            "error": {
                "kind": diagnostic_kind(diag),
                "message": diag.to_string(),
            },
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn auth_login(&mut self, outcome: &AuthLoginOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": {
                "user": outcome.user,
                "expires_at": outcome.expires_at.map(|dt| dt.to_rfc3339()),
                "state": "login_success",
            },
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn auth_status(&mut self, outcome: &AuthStatusOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
        let obj = match outcome {
            AuthStatusOutcome::LoggedIn { server: _, user, expires_at, expired } => json!({
                "_meta": meta_block(meta, 0),
                "data": {
                    "user": user,
                    "expires_at": expires_at.map(|dt| dt.to_rfc3339()),
                    "state": if *expired { "expired" } else { "logged_in" },
                },
            }),
            // DSP_TOKEN env-override: data shape is uniform with the LoggedIn case
            // (user: null, expires_at: rfc3339 or null, state: "logged_in"|"expired").
            // The "via DSP_TOKEN" disclosure is carried by _meta.auth, not by a
            // source key in data, to keep the data shape stable across all three outcomes.
            AuthStatusOutcome::AuthenticatedViaEnv { server: _, expires_at, expired } => json!({
                "_meta": meta_block(meta, 0),
                "data": {
                    "user": null,
                    "expires_at": expires_at.map(|dt| dt.to_rfc3339()),
                    "state": if *expired { "expired" } else { "logged_in" },
                },
            }),
            AuthStatusOutcome::NotLoggedIn { server: _ } => json!({
                "_meta": meta_block(meta, 0),
                "data": {
                    "user": null,
                    "expires_at": null,
                    "state": "not_logged_in",
                },
            }),
        };
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn auth_logout(&mut self, outcome: &AuthLogoutOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": {
                "was_cached": outcome.was_cached,
                "state": if outcome.was_cached { "logout_was_cached" } else { "logout_no_op" },
            },
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn auth_set_token(&mut self, outcome: &AuthSetTokenOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": {
                "user": outcome.user,
                "expires_at": outcome.expires_at.map(|dt| dt.to_rfc3339()),
                "state": "token_cached",
            },
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn project_dump(&mut self, outcome: &DumpOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": {
                "path": outcome.path.display().to_string(),
                "bytes": outcome.bytes,
                "cleaned_up": outcome.cleaned_up,
                "reused": outcome.reused,
                "created_at": outcome.created_at.map(|dt| dt.to_rfc3339()),
            },
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn project_dump_deleted(&mut self, outcome: &DumpDeleteOutcome, meta: &MetaContext) -> Result<(), Diagnostic> {
        let obj = if let Some(ref note) = outcome.note {
            json!({
                "_meta": meta_block(meta, 0),
                "data": {
                    "deleted": outcome.deleted,
                    "note": note,
                },
            })
        } else {
            json!({
                "_meta": meta_block(meta, 0),
                "data": {
                    "deleted": outcome.deleted,
                },
            })
        };
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn projects(&mut self, view: &ProjectListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Build data array. Each element uses `json!` with insertion-order keys
        // (preserve_order feature on serde_json, per dsp-cli/ADR-0003).
        // longname None → JSON null.
        let data: Vec<serde_json::Value> = view
            .items
            .iter()
            .map(|item| {
                json!({
                    "iri": item.iri,
                    "shortcode": item.shortcode,
                    "shortname": item.shortname,
                    "longname": item.longname,
                    "status": item.status.as_str(),
                    "data_models": item.data_models,
                })
            })
            .collect();

        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": data,
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn project_describe(&mut self, project: &ProjectDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // `data` is a single object (dsp-cli/ADR-0003). Deterministic key order via `json!`
        // (preserve_order feature ensures insertion order).
        let description: Vec<serde_json::Value> = project
            .description
            .iter()
            .map(|d| {
                json!({
                    "value": d.value,
                    "language": d.language,
                })
            })
            .collect();

        let data_models: Vec<serde_json::Value> = project
            .data_models
            .iter()
            .map(|dm| {
                json!({
                    "name": dm.name,
                    "iri": dm.iri,
                })
            })
            .collect();

        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": {
                "iri": project.iri,
                "shortcode": project.shortcode,
                "shortname": project.shortname,
                "longname": project.longname,
                "status": project.status.as_str(),
                "description": description,
                "keywords": project.keywords,
                "data_models": data_models,
            },
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn data_model_describe(&mut self, detail: &DataModelDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // dsp-cli/ADR-0003 single-object envelope. `last_modified` is the full RFC3339
        // string (lossless). `resource_types` is an array of per-resource-type
        // objects (name, iri, label).
        let resource_types: Vec<serde_json::Value> = detail
            .resource_types
            .iter()
            .map(|rt| {
                json!({
                    "name": rt.name,
                    "iri": rt.iri,
                    "label": rt.label,
                })
            })
            .collect();

        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": {
                "name": detail.name,
                "iri": detail.iri,
                "label": detail.label,
                "last_modified": detail.last_modified,
                "resource_types": resource_types,
            },
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn data_models(&mut self, view: &DataModelListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Build data array — per-item key order via `json!` insertion order
        // (preserve_order feature on serde_json, per dsp-cli/ADR-0003).
        // label None → JSON null; last_modified None → JSON null; is_builtin → bool.
        let data: Vec<serde_json::Value> = view
            .items
            .iter()
            .map(|item| {
                json!({
                    "name": item.name,
                    "iri": item.iri,
                    "label": item.label,
                    "last_modified": item.last_modified,
                    "is_builtin": item.is_builtin,
                })
            })
            .collect();

        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": data,
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn resource_types(&mut self, view: &ResourceTypeListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Build data array — per-item key order via `json!` insertion order
        // (preserve_order feature on serde_json, per dsp-cli/ADR-0003).
        // label None → JSON null; is_builtin → bool.
        //
        // `count` (plan 030) is DELIBERATELY omitted (not emitted as `null`)
        // when the item carries no count — unlike `label`'s always-present
        // null, this keeps `--count`-less output (the only case exercised by
        // today's fixtures/snapshots, since no caller sets `count` yet) byte-
        // identical to pre-030 output. See design plan 030-resource-type-count in the
        // dsp-incubator archive.
        let data: Vec<serde_json::Value> = view
            .items
            .iter()
            .map(|item| {
                let mut obj = json!({
                    "name": item.name,
                    "iri": item.iri,
                    "label": item.label,
                    "is_builtin": item.is_builtin,
                });
                if let Some(count) = item.count {
                    obj["count"] = serde_json::Value::from(count);
                }
                obj
            })
            .collect();

        // D3-style: add `note` to _meta when count_caveat is Some (plan 030).
        let mut meta_obj = meta_block(meta, 0);
        if let Some(ref cc) = meta.count_caveat {
            meta_obj["note"] = serde_json::Value::from(cc.as_str());
        }

        let obj = json!({
            "_meta": meta_obj,
            "data": data,
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn resource_type_describe(
        &mut self,
        detail: &crate::model::ResourceTypeDetail,
        meta: &MetaContext,
    ) -> Result<(), Diagnostic> {
        // dsp-cli/ADR-0003 single-object envelope. `_meta` first, `data` is the resource-type
        // object. Fields array carries one object per field (name, iri, label,
        // value_type, link_target, cardinality, is_builtin, data_model).
        let fields: Vec<serde_json::Value> = detail
            .fields
            .iter()
            .map(|f| {
                json!({
                    "name": f.name,
                    "iri": f.iri,
                    "label": f.label,
                    "value_type": f.value_type.to_string(),
                    "link_target": f.link_target,
                    "cardinality": f.cardinality.to_string(),
                    "is_builtin": f.is_builtin,
                    "data_model": f.data_model,
                })
            })
            .collect();

        // D3-style: add `note` to _meta when count_caveat is Some (plan 030).
        let mut meta_obj = meta_block(meta, 0);
        if let Some(ref cc) = meta.count_caveat {
            meta_obj["note"] = serde_json::Value::from(cc.as_str());
        }

        // `count` (plan 030) is DELIBERATELY omitted (not emitted as `null`)
        // when `detail.count` is `None` — keeps `--count`-less output (the
        // only case exercised by today's fixtures/snapshots) byte-identical
        // to pre-030 output. See design plan 030-resource-type-count in the dsp-incubator
        // archive.
        let mut data = json!({
            "name": detail.name,
            "iri": detail.iri,
            "label": detail.label,
            "data_model": detail.data_model,
            "representation": detail.representation.as_ref().map(|r| r.to_string()),
            "super_types": detail.super_types,
            "fields": fields,
        });
        if let Some(count) = detail.count {
            data["count"] = serde_json::Value::from(count);
        }

        let obj = json!({
            "_meta": meta_obj,
            "data": data,
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn data_model_structure(&mut self, structure: &DataModelStructure, meta: &MetaContext) -> Result<(), Diagnostic> {
        // dsp-cli/ADR-0003 flat-array envelope. Each element carries all 5 keys (none omitted).
        // Optional values are emitted as JSON null (matching resource_type_describe lines
        // 476-485 which render None Options as null — never skip_serializing_if).
        let data: Vec<serde_json::Value> = structure
            .relations
            .iter()
            .map(|r| {
                json!({
                    "source": r.source,
                    "target": r.target,
                    "kind": r.kind.to_string(),
                    "field": r.field,
                    "target_data_model": r.target_data_model,
                })
            })
            .collect();

        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": data,
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn resources(&mut self, view: &ResourceListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Build data array.
        let data: Vec<serde_json::Value> = view
            .items
            .iter()
            .map(|item| {
                json!({
                    "label": item.label,
                    "iri": item.iri,
                    "ark_url": item.ark_url,
                    "creation_date": item.creation_date,
                    "last_modified": item.last_modified,
                    "resource_type": item.resource_type,
                })
            })
            .collect();

        // Build _meta pagination keys (D5 — two asymmetric shapes by mode).
        let mut meta_obj = meta_block(meta, 0);
        match &view.pagination {
            ResourceListPagination::SinglePage { page, may_have_more } => {
                meta_obj["page"] = serde_json::Value::from(*page);
                meta_obj["may_have_more_results"] = serde_json::Value::from(*may_have_more);
            }
            ResourceListPagination::AllPages { pages_fetched } => {
                meta_obj["pages_fetched"] = serde_json::Value::from(*pages_fetched);
                // AllPages always exits on may_have_more_results = false (loop invariant).
                meta_obj["may_have_more_results"] = serde_json::Value::from(false);
            }
        }

        // D3: add `note` to _meta when filter_warning is Some.
        if let Some(ref fw) = meta.filter_warning {
            meta_obj["note"] = serde_json::Value::from(fw.as_str());
        }

        let obj = json!({
            "_meta": meta_obj,
            "data": data,
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn resource_describe(&mut self, detail: &ResourceDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // dsp-cli/ADR-0003 single-object envelope. `data` is an object (not array).
        // Keys in deterministic order; `None` → JSON null.
        // D3: add `note` to _meta when filter_warning is Some.
        let mut meta_obj = meta_block(meta, 0);
        if let Some(ref fw) = meta.filter_warning {
            meta_obj["note"] = serde_json::Value::from(fw.as_str());
        }

        // Build data object with explicit key ordering (preserve_order, dsp-cli/ADR-0003).
        let mut data = serde_json::Map::new();
        data.insert("label".into(), serde_json::Value::String(detail.label.clone()));
        data.insert("iri".into(), serde_json::Value::String(detail.iri.clone()));
        data.insert("resource_type".into(), serde_json::Value::String(detail.resource_type.clone()));
        data.insert(
            "ark_url".into(),
            detail
                .ark_url
                .as_ref()
                .map_or(serde_json::Value::Null, |s| serde_json::Value::String(s.clone())),
        );
        data.insert(
            "creation_date".into(),
            detail
                .creation_date
                .as_ref()
                .map_or(serde_json::Value::Null, |s| serde_json::Value::String(s.clone())),
        );
        data.insert(
            "last_modified".into(),
            detail
                .last_modified
                .as_ref()
                .map_or(serde_json::Value::Null, |s| serde_json::Value::String(s.clone())),
        );
        data.insert(
            "attached_project".into(),
            detail
                .attached_project
                .as_ref()
                .map_or(serde_json::Value::Null, |s| serde_json::Value::String(s.clone())),
        );
        data.insert(
            "owner".into(),
            detail
                .owner
                .as_ref()
                .map_or(serde_json::Value::Null, |s| serde_json::Value::String(s.clone())),
        );
        data.insert(
            "visibility".into(),
            detail
                .visibility
                .as_ref()
                .map_or(serde_json::Value::Null, |v| serde_json::Value::String(v.as_str().into())),
        );
        data.insert(
            "your_access".into(),
            detail
                .your_access
                .as_ref()
                .map_or(serde_json::Value::Null, |a| serde_json::Value::String(a.as_str().into())),
        );
        // `values` key is present only when --values was set (detail.values is Some).
        // Absent (not null) when None — preserves 8b envelope byte-for-byte.
        if let Some(ref fields) = detail.values {
            let values_arr: Vec<serde_json::Value> = fields
                .iter()
                .map(|fv| {
                    let value_objs: Vec<serde_json::Value> = fv
                        .values
                        .iter()
                        .map(|v| {
                            let mut obj = value_content_to_json(&v.content);
                            if let (Some(c), serde_json::Value::Object(m)) = (&v.comment, &mut obj) {
                                m.insert("comment".into(), serde_json::Value::String(c.clone()));
                            }
                            obj
                        })
                        .collect();
                    let mut fg = serde_json::Map::new();
                    fg.insert("field".into(), serde_json::Value::String(fv.name.clone()));
                    let fl_val = match &fv.label {
                        Some(s) => serde_json::Value::String(s.clone()),
                        None => serde_json::Value::Null,
                    };
                    fg.insert("field_label".into(), fl_val);
                    fg.insert("values".into(), serde_json::Value::Array(value_objs));
                    serde_json::Value::Object(fg)
                })
                .collect();
            data.insert("values".into(), serde_json::Value::Array(values_arr));
        }

        let obj = json!({
            "_meta": meta_obj,
            "data": serde_json::Value::Object(data),
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn vocabularies(&mut self, view: &VocabularyListView, meta: &MetaContext) -> Result<(), Diagnostic> {
        // Build data array. `labels`/`comments` are the verbatim, lossless
        // `{value, language}` arrays (json stays lossless — no per-language
        // column collapsing, unlike tabular). `nodes`/`depth` (plan 034) are
        // DELIBERATELY omitted (not emitted as null) when the item carries no
        // count — mirrors `resource_types`'s `count` omission convention.
        let data: Vec<serde_json::Value> = view
            .items
            .iter()
            .map(|item| {
                let labels = localized_text_array(&item.header.labels);
                let comments = localized_text_array(&item.header.comments);
                let mut obj = json!({
                    "name": item.header.name,
                    "iri": item.header.iri,
                    "labels": labels,
                    "comments": comments,
                });
                if let Some(n) = item.node_count {
                    obj["nodes"] = serde_json::Value::from(n);
                }
                if let Some(d) = item.depth {
                    obj["depth"] = serde_json::Value::from(d);
                }
                obj
            })
            .collect();

        // D3-style: add `note` to _meta when count_cost is Some (plan 034;
        // mirrors the count_caveat assignment above for `resource_types`).
        let mut meta_obj = meta_block(meta, 0);
        if let Some(ref cc) = meta.count_cost {
            meta_obj["note"] = serde_json::Value::from(cc.as_str());
        }

        let obj = json!({
            "_meta": meta_obj,
            "data": data,
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }

    fn vocabulary_describe(&mut self, detail: &VocabularyDetail, meta: &MetaContext) -> Result<(), Diagnostic> {
        // dsp-cli/ADR-0003 single-object envelope. `nodes`/`depth` are ALWAYS present
        // here (plain `usize` on `VocabularyDetail`, unlike `list`'s Option) —
        // the omit-when-absent rule above does not apply.
        //
        // Per-node array key: named `"children"` (not `"nodes"`, which would
        // collide with the top-level node-count key `data.nodes`) — chosen to
        // match the model's own `VocabularyTree.children`/`VocabularyNode.children`
        // naming: json's `data.children` IS a nested tree, mirroring the
        // model shape one-to-one (plan 034 review fix — json is the lossless
        // path, so it renders the tree AS a tree rather than flattening it
        // like the tabular formats do). `path`/`depth`/`parent_iri` are
        // dropped here (structural/derivable from nesting); `number` and
        // `position` are kept.
        fn nested_to_json(node: &NestedVocabularyNode<'_>) -> serde_json::Value {
            let h = node.header;
            json!({
                "node_iri": h.iri,
                "number": node.number,
                "name": h.name,
                "labels": localized_text_array(&h.labels),
                "comments": localized_text_array(&h.comments),
                "position": node.position,
                "children": node.children.iter().map(nested_to_json).collect::<Vec<_>>(),
            })
        }

        let root = &detail.tree.root;
        let children: Vec<serde_json::Value> =
            nest_vocabulary_detail(detail).iter().map(nested_to_json).collect::<Vec<_>>();

        let mut data = json!({
            "name": root.name,
            "iri": root.iri,
            "labels": localized_text_array(&root.labels),
            "comments": localized_text_array(&root.comments),
            "nodes": detail.node_count,
            "depth": detail.depth,
            "children": children,
        });
        // `requested_node`/`subtree_of`: omitted (not null) when not
        // applicable — there is no separate boolean, `subtree_of` being
        // present IS the `--subtree` flag.
        if let Some(ref requested) = detail.tree.requested_node {
            data["requested_node"] = serde_json::Value::from(requested.as_str());
        }
        if let Some(ref subtree_of) = detail.subtree_of {
            data["subtree_of"] = serde_json::Value::from(subtree_of.as_str());
        }

        let obj = json!({
            "_meta": meta_block(meta, 0),
            "data": data,
        });
        writeln!(
            self.out,
            "{}",
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        DataModel, DataModelDetail, DataModelSummary, Project, ProjectDescription, ProjectDetail, ProjectStatus,
        ResourceType, ResourceTypeSummary,
    };
    use crate::render::test_support::{SharedBuf, make_meta};
    use crate::render::{DataModelListView, ResourceTypeListView};

    #[test]
    fn meta_block_full_context_has_all_three_keys() {
        // Non-empty server_label/auth_state (the shape every success path uses)
        // → server, auth, and exit_code all present.
        let meta = MetaContext {
            server_label: "https://api.test.dasch.swiss".into(),
            auth_state: "anonymous".into(),
            filter_warning: None,
            count_caveat: None,
            count_cost: None,
        };
        let block = meta_block(&meta, 1);
        let obj = block.as_object().unwrap();
        assert_eq!(obj["server"], "https://api.test.dasch.swiss");
        assert_eq!(obj["auth"], "anonymous");
        assert_eq!(obj["exit_code"], 1);
        assert_eq!(obj.len(), 3, "expected exactly three keys; got: {block}");
    }

    #[test]
    fn meta_block_empty_server_and_auth_omits_both_keys() {
        // Empty server_label/auth_state (the top-level-error shape, plan 032 D3)
        // → only exit_code is present; server/auth are absent (not null).
        let meta = MetaContext {
            server_label: String::new(),
            auth_state: String::new(),
            filter_warning: None,
            count_caveat: None,
            count_cost: None,
        };
        let block = meta_block(&meta, 2);
        let obj = block.as_object().unwrap();
        assert!(
            !obj.contains_key("server"),
            "server key must be absent when server_label is empty; got: {block}"
        );
        assert!(
            !obj.contains_key("auth"),
            "auth key must be absent when auth_state is empty; got: {block}"
        );
        assert_eq!(obj["exit_code"], 2);
        assert_eq!(obj.len(), 1, "expected exactly one key; got: {block}");
    }

    #[test]
    fn projects_json_output() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let items = vec![
            Project {
                iri: "http://rdfh.ch/projects/0001".into(),
                shortcode: "0001".into(),
                shortname: "anything".into(),
                longname: Some("Anything Project".into()),
                status: ProjectStatus::Active,
                data_models: 2,
            },
            Project {
                iri: "http://rdfh.ch/projects/0002".into(),
                shortcode: "0002".into(),
                shortname: "images".into(),
                longname: None,
                status: ProjectStatus::Inactive,
                data_models: 0,
            },
        ];
        let view = ProjectListView { items, total: 2, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();

        let s = out.string();
        let parsed: serde_json::Value = serde_json::from_str(s.trim()).unwrap();

        // _meta present
        assert_eq!(parsed["_meta"]["auth"], "anonymous");
        assert_eq!(parsed["_meta"]["server"], "https://api.test.dasch.swiss");
        assert_eq!(parsed["_meta"]["exit_code"], 0);

        // data is array
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data.len(), 2);

        // first item
        assert_eq!(data[0]["shortcode"], "0001");
        assert_eq!(data[0]["shortname"], "anything");
        assert_eq!(data[0]["longname"], "Anything Project");
        assert_eq!(data[0]["status"], "active");
        assert_eq!(data[0]["data_models"], 2);
        assert_eq!(data[0]["iri"], "http://rdfh.ch/projects/0001");

        // second item — longname None → null
        assert_eq!(data[1]["shortcode"], "0002");
        assert!(data[1]["longname"].is_null());
        assert_eq!(data[1]["status"], "inactive");
        assert_eq!(data[1]["data_models"], 0);
    }

    #[test]
    fn projects_json_empty_data_array() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let view = ProjectListView { items: vec![], total: 0, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.projects(&view, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        assert!(parsed["data"].as_array().unwrap().is_empty());
    }

    /// Assert that `diagnostic_kind` returns the correct stable string for every
    /// `Diagnostic` variant. This test is intentionally exhaustive: adding a new
    /// variant without a corresponding arm in `diagnostic_kind` will cause a
    /// compiler warning (non-exhaustive match) at the match site, but this test
    /// ensures the mapping is also exercised at the call level so the kind string
    /// is verified, not just the pattern.
    #[test]
    fn diagnostic_kind_covers_all_variants() {
        let cases: &[(&Diagnostic, &str)] = &[
            (&Diagnostic::Usage("x".into()), "usage"),
            (&Diagnostic::AuthRequired("x".into()), "auth_required"),
            (&Diagnostic::NotFound("x".into()), "not_found"),
            (&Diagnostic::ServerError("x".into()), "server_error"),
            (&Diagnostic::Network("x".into()), "network"),
            (&Diagnostic::Conflict("x".into()), "conflict"),
            (&Diagnostic::Io("x".into()), "io"),
            (&Diagnostic::Internal("x".into()), "internal"),
            (&Diagnostic::NotImplemented("x".into()), "internal"),
        ];
        for (diag, expected_kind) in cases {
            assert_eq!(diagnostic_kind(diag), *expected_kind, "unexpected kind for {diag:?}");
        }
    }

    #[test]
    fn conflict_kind_is_conflict() {
        let d = Diagnostic::Conflict("dump already in progress".into());
        assert_eq!(diagnostic_kind(&d), "conflict");
    }

    #[test]
    fn io_kind_is_io() {
        let d = Diagnostic::Io("failed to write /tmp/0001.zip: permission denied".into());
        assert_eq!(diagnostic_kind(&d), "io");
    }

    /// Pins the `kind` / `_meta.exit_code` mapping for a NON-usage `Diagnostic`
    /// via `JsonRenderer::diagnostic` (plan 032's Test plan). The binary-level
    /// tests in `tests/cli.rs` only exercise usage errors (exit code 2, no
    /// server needed); this test covers a Runtime-category kind (exit code 1)
    /// generically, without a server, so a hard-to-trigger non-network exit-3
    /// case isn't required.
    #[test]
    fn diagnostic_not_found_json_output() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let diag = Diagnostic::NotFound("resource http://rdfh.ch/0001/xyz not found".into());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.diagnostic(&diag, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        assert_eq!(parsed["error"]["kind"], "not_found");
        assert_eq!(parsed["_meta"]["exit_code"], 1);
        let message = parsed["error"]["message"].as_str().expect("error.message must be a string");
        assert!(!message.is_empty(), "error.message must be non-empty");
        assert_eq!(message, diag.to_string());
    }

    fn make_beol_detail() -> ProjectDetail {
        // Four data-models passed in already-sorted order (client layer sorts;
        // renderer passes them through as-is). Having all four here guards against
        // a renderer that truncates or reorders the slice.
        ProjectDetail {
            iri: "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF".into(),
            shortcode: "0801".into(),
            shortname: "beol".into(),
            longname: Some("Bernoulli-Euler Online".into()),
            status: ProjectStatus::Active,
            description: vec![ProjectDescription {
                value: "<b>BEOL</b> — early modern mathematics.".into(),
                language: Some("en".into()),
            }],
            keywords: vec!["Bernoulli".into(), "Euler".into(), "Mathematics".into()],
            data_models: vec![
                DataModelSummary {
                    name: "beol".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
                },
                DataModelSummary {
                    name: "biblio".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".into(),
                },
                DataModelSummary {
                    name: "leibniz".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/leibniz/v2".into(),
                },
                DataModelSummary {
                    name: "newton".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/newton/v2".into(),
                },
            ],
        }
    }

    #[test]
    fn project_describe_json_full() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&make_beol_detail(), &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();

        // _meta present
        assert_eq!(parsed["_meta"]["auth"], "anonymous");
        assert_eq!(parsed["_meta"]["exit_code"], 0);

        // data is a single object
        let data = &parsed["data"];
        assert!(data.is_object());
        assert_eq!(data["iri"], "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF");
        assert_eq!(data["shortcode"], "0801");
        assert_eq!(data["shortname"], "beol");
        assert_eq!(data["longname"], "Bernoulli-Euler Online");
        assert_eq!(data["status"], "active");

        // description array
        let desc = data["description"].as_array().unwrap();
        assert_eq!(desc.len(), 1);
        assert_eq!(desc[0]["value"], "<b>BEOL</b> — early modern mathematics.");
        assert_eq!(desc[0]["language"], "en");

        // keywords array
        let kws = data["keywords"].as_array().unwrap();
        assert_eq!(kws.len(), 3);
        assert_eq!(kws[0], "Bernoulli");

        // data_models array — four entries in sorted order (beol, biblio, leibniz, newton).
        // Guards against a renderer that truncates or reorders the slice.
        let dms = data["data_models"].as_array().unwrap();
        assert_eq!(dms.len(), 4, "all four data-models must be rendered");
        assert_eq!(dms[0]["name"], "beol");
        assert_eq!(dms[0]["iri"], "http://api.dasch.swiss/ontology/0801/beol/v2");
        assert_eq!(dms[1]["name"], "biblio");
        assert_eq!(dms[2]["name"], "leibniz");
        assert_eq!(dms[3]["name"], "newton");
    }

    #[test]
    fn project_describe_json_no_longname() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_beol_detail();
        detail.longname = None;
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        // longname None → JSON null
        assert!(parsed["data"]["longname"].is_null());
    }

    #[test]
    fn project_describe_json_empty_fields() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0000".into(),
            shortcode: "0000".into(),
            shortname: "minimal".into(),
            longname: None,
            status: ProjectStatus::Inactive,
            description: vec![],
            keywords: vec![],
            data_models: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.project_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let data = &parsed["data"];
        assert_eq!(data["status"], "inactive");
        assert!(data["description"].as_array().unwrap().is_empty());
        assert!(data["keywords"].as_array().unwrap().is_empty());
        assert!(data["data_models"].as_array().unwrap().is_empty());
    }

    fn make_data_model_fixture() -> Vec<DataModel> {
        vec![
            DataModel {
                name: "beol".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
                label: Some("The BEOL data-model".into()),
                last_modified: Some("2024-05-27T13:43:26.233048Z".into()),
                is_builtin: false,
            },
            DataModel {
                name: "biblio".into(),
                iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".into(),
                label: None,
                last_modified: None,
                is_builtin: false,
            },
            DataModel {
                name: "knora-api".into(),
                iri: "http://api.knora.org/ontology/knora-api/v2".into(),
                label: None,
                last_modified: None,
                is_builtin: true,
            },
        ]
    }

    #[test]
    fn data_models_json_output() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let view = DataModelListView { items: make_data_model_fixture(), total: 3, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();

        // _meta present
        assert_eq!(parsed["_meta"]["auth"], "anonymous");
        assert_eq!(parsed["_meta"]["server"], "https://api.test.dasch.swiss");
        assert_eq!(parsed["_meta"]["exit_code"], 0);

        // data is an array
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data.len(), 3);

        // first item: has label and last_modified, not builtin
        assert_eq!(data[0]["name"], "beol");
        assert_eq!(data[0]["iri"], "http://api.dasch.swiss/ontology/0801/beol/v2");
        assert_eq!(data[0]["label"], "The BEOL data-model");
        assert_eq!(data[0]["last_modified"], "2024-05-27T13:43:26.233048Z");
        assert_eq!(data[0]["is_builtin"], false);

        // second item: label None → null, last_modified None → null
        assert_eq!(data[1]["name"], "biblio");
        assert!(data[1]["label"].is_null());
        assert!(data[1]["last_modified"].is_null());
        assert_eq!(data[1]["is_builtin"], false);

        // third item: builtin, both null
        assert_eq!(data[2]["name"], "knora-api");
        assert!(data[2]["label"].is_null());
        assert!(data[2]["last_modified"].is_null());
        assert_eq!(data[2]["is_builtin"], true);
    }

    #[test]
    fn data_models_json_empty_data_array() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let view = DataModelListView { items: vec![], total: 0, filter: None };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_models(&view, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        assert!(parsed["data"].as_array().unwrap().is_empty());
    }

    // ── data_model_describe JSON tests ────────────────────────────────────────

    fn make_beol_dm_detail() -> DataModelDetail {
        DataModelDetail {
            name: "beol".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
            label: Some("The BEOL data-model".into()),
            last_modified: Some("2024-05-27T13:43:26.233048Z".into()),
            resource_types: vec![
                ResourceTypeSummary {
                    name: "Archive".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#Archive".into(),
                    label: Some("Archive".into()),
                },
                ResourceTypeSummary {
                    name: "basicLetter".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#basicLetter".into(),
                    label: None,
                },
                ResourceTypeSummary {
                    name: "letter".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
                    label: Some("Letter".into()),
                },
            ],
        }
    }

    #[test]
    fn data_model_describe_json_full() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_model_describe(&make_beol_dm_detail(), &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();

        // _meta present
        assert_eq!(parsed["_meta"]["auth"], "anonymous");
        assert_eq!(parsed["_meta"]["server"], "https://api.test.dasch.swiss");
        assert_eq!(parsed["_meta"]["exit_code"], 0);

        // data is a single object (dsp-cli/ADR-0003)
        let data = &parsed["data"];
        assert!(data.is_object());
        assert_eq!(data["name"], "beol");
        assert_eq!(data["iri"], "http://api.dasch.swiss/ontology/0801/beol/v2");
        assert_eq!(data["label"], "The BEOL data-model");
        // last_modified is the full RFC3339 string (lossless)
        assert_eq!(data["last_modified"], "2024-05-27T13:43:26.233048Z");

        // resource_types array
        let rts = data["resource_types"].as_array().unwrap();
        assert_eq!(rts.len(), 3);
        assert_eq!(rts[0]["name"], "Archive");
        assert_eq!(rts[0]["iri"], "http://api.dasch.swiss/ontology/0801/beol/v2#Archive");
        assert_eq!(rts[0]["label"], "Archive");
        // label None → null
        assert_eq!(rts[1]["name"], "basicLetter");
        assert!(rts[1]["label"].is_null());
        assert_eq!(rts[2]["name"], "letter");
        assert_eq!(rts[2]["label"], "Letter");
    }

    #[test]
    fn data_model_describe_json_no_label_no_last_modified() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let detail = DataModelDetail {
            name: "minimal".into(),
            iri: "http://api.dasch.swiss/ontology/0000/minimal/v2".into(),
            label: None,
            last_modified: None,
            resource_types: vec![],
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.data_model_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let data = &parsed["data"];
        // label None → null
        assert!(data["label"].is_null());
        // last_modified None → null
        assert!(data["last_modified"].is_null());
        // resource_types empty array
        assert!(data["resource_types"].as_array().unwrap().is_empty());
    }

    // ── resource_types JSON tests ─────────────────────────────────────────────

    fn make_rt_fixture() -> Vec<ResourceType> {
        vec![
            ResourceType {
                name: "Archive".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#Archive".into(),
                label: Some("Archive".into()),
                is_builtin: false,
                count: None,
            },
            ResourceType {
                name: "letter".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
                label: None,
                is_builtin: false,
                count: None,
            },
        ]
    }

    #[test]
    fn resource_types_json_output() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: make_rt_fixture(),
            total: 2,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();

        // _meta present
        assert_eq!(parsed["_meta"]["auth"], "anonymous");
        assert_eq!(parsed["_meta"]["server"], "https://api.test.dasch.swiss");
        assert_eq!(parsed["_meta"]["exit_code"], 0);

        // data is an array
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data.len(), 2);

        // first item: has label, not builtin
        assert_eq!(data[0]["name"], "Archive");
        assert_eq!(data[0]["iri"], "http://api.dasch.swiss/ontology/0801/beol/v2#Archive");
        assert_eq!(data[0]["label"], "Archive");
        assert_eq!(data[0]["is_builtin"], false);

        // second item: label None → null
        assert_eq!(data[1]["name"], "letter");
        assert!(data[1]["label"].is_null());
        assert_eq!(data[1]["is_builtin"], false);
    }

    #[test]
    fn resource_types_json_with_builtins() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: vec![ResourceType {
                name: "Region".into(),
                iri: "http://api.knora.org/ontology/knora-api/v2#Region".into(),
                label: Some("Region".into()),
                is_builtin: true,
                count: None,
            }],
            total: 1,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["name"], "Region");
        assert_eq!(data[0]["label"], "Region");
        // is_builtin must be true (a bool, not a string)
        assert_eq!(data[0]["is_builtin"], true);
    }

    #[test]
    fn resource_types_json_with_filter() {
        // filter does not affect JSON output shape; just verify it renders cleanly
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: vec![make_rt_fixture().remove(0)], // just Archive
            total: 2,
            filter: Some("arch".to_string()),
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["name"], "Archive");
    }

    #[test]
    fn resource_types_json_empty_data_array() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: vec![],
            total: 0,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        assert!(parsed["data"].as_array().unwrap().is_empty());
    }

    #[test]
    fn resource_types_json_with_count_and_caveat() {
        // plan 030: `count` key present (numeric) when Some, `_meta.note`
        // carries `count_caveat` when Some.
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut items = make_rt_fixture();
        items[0].count = Some(5);
        let view = ResourceTypeListView { items, total: 2, filter: None, data_model: "beol".into() };
        let meta = crate::render::MetaContext {
            server_label: "https://api.test.dasch.swiss".into(),
            auth_state: "anonymous".into(),
            filter_warning: None,
            count_caveat: Some("counts are not permission-filtered".into()),
            count_cost: None,
        };
        renderer.resource_types(&view, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        assert_eq!(parsed["data"][0]["count"], 5);
        // Second item has no count → key absent (not null).
        assert!(
            !parsed["data"][1].as_object().unwrap().contains_key("count"),
            "count key must be absent when None; got: {}",
            parsed["data"][1]
        );
        assert_eq!(
            parsed["_meta"]["note"], "counts are not permission-filtered",
            "note must carry count_caveat"
        );
    }

    #[test]
    fn resource_types_json_no_count_no_note() {
        // Regression: count_caveat: None (today's only production case) →
        // no `note` key, no `count` key on any item.
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let view = ResourceTypeListView {
            items: make_rt_fixture(),
            total: 2,
            filter: None,
            data_model: "beol".into(),
        };
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_types(&view, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        assert!(!parsed["_meta"].as_object().unwrap().contains_key("note"));
        assert!(!parsed["data"][0].as_object().unwrap().contains_key("count"));
    }

    // ── resource_type_describe JSON tests ─────────────────────────────────────

    fn make_minimal_rt_detail() -> crate::model::ResourceTypeDetail {
        crate::model::ResourceTypeDetail {
            name: "manuscript".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#manuscript".into(),
            label: Some("Manuscript".into()),
            data_model: "beol".into(),
            representation: None,
            super_types: vec![],
            fields: vec![],
            count: None,
        }
    }

    #[test]
    fn resource_type_describe_json_with_count_and_caveat() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_minimal_rt_detail();
        detail.count = Some(99);
        let meta = crate::render::MetaContext {
            server_label: "https://api.dasch.swiss".into(),
            auth_state: "anonymous".into(),
            filter_warning: None,
            count_caveat: Some("counts exclude deleted resources".into()),
            count_cost: None,
        };
        renderer.resource_type_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        assert_eq!(parsed["data"]["count"], 99);
        assert_eq!(
            parsed["_meta"]["note"], "counts exclude deleted resources",
            "note must carry count_caveat"
        );
    }

    #[test]
    fn resource_type_describe_json_no_count_no_note() {
        // Regression: count: None / count_caveat: None (today's only
        // production case) → no `count` key, no `note` key.
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let meta = make_meta("anonymous", "https://api.dasch.swiss");
        renderer.resource_type_describe(&make_minimal_rt_detail(), &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        assert!(!parsed["data"].as_object().unwrap().contains_key("count"));
        assert!(!parsed["_meta"].as_object().unwrap().contains_key("note"));
    }

    // ── resource_describe JSON values tests ───────────────────────────────────

    use crate::model::resource_type::ValueType;
    use crate::model::{
        DatePoint, DateValue, FieldValues, FileValue, ResourceAccess, ResourceDetail, ResourceVisibility, Value,
        ValueContent,
    };

    fn make_resource_detail_no_values() -> ResourceDetail {
        ResourceDetail {
            label: "Test Resource".into(),
            iri: "http://rdfh.ch/0803/abc123".into(),
            resource_type: "Page".into(),
            ark_url: None,
            creation_date: None,
            last_modified: None,
            attached_project: None,
            owner: None,
            visibility: Some(ResourceVisibility::Public),
            your_access: Some(ResourceAccess::View),
            values: None,
        }
    }

    #[test]
    fn resource_describe_json_no_values_key_absent() {
        // When values is None, the "values" key must be ABSENT (not null) in json output.
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&make_resource_detail_no_values(), &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let data = &parsed["data"];
        assert!(
            data["values"].is_null() && !data.as_object().unwrap().contains_key("values"),
            "values key must be absent when values is None; got data: {data}"
        );
    }

    #[test]
    fn resource_describe_json_some_values_array_present() {
        // When values is Some, the "values" key must be present in json data.
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "hasTitle".into(),
            label: Some("Title".into()),
            values: vec![ValueContent::Text("Hello".into()).into()],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let data = &parsed["data"];
        assert!(
            data.as_object().unwrap().contains_key("values"),
            "values key must be present when values is Some; got data: {data}"
        );
        let values = data["values"].as_array().unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0]["field"], "hasTitle");
        assert_eq!(values[0]["field_label"], "Title");
        let val_objs = values[0]["values"].as_array().unwrap();
        assert_eq!(val_objs.len(), 1);
        assert_eq!(val_objs[0]["value_type"], "text");
        assert_eq!(val_objs[0]["text"], "Hello");
    }

    #[test]
    fn resource_describe_json_some_empty_values_array() {
        // Some(vec![]) → "values": [] — key present, empty array.
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let data = &parsed["data"];
        assert!(
            data.as_object().unwrap().contains_key("values"),
            "values key must be present even for empty Some(vec![]);"
        );
        assert!(data["values"].as_array().unwrap().is_empty());
    }

    #[test]
    fn resource_describe_json_field_label_null_when_none() {
        // field_label null when label is None.
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "seqnum".into(),
            label: None,
            values: vec![ValueContent::Integer(42).into()],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let fg = &parsed["data"]["values"][0];
        assert!(fg["field_label"].is_null(), "field_label must be null when label is None");
        let val = &fg["values"][0];
        assert_eq!(val["value_type"], "integer");
        assert_eq!(val["value"], 42);
    }

    #[test]
    fn resource_describe_json_link_value() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "isPartOf".into(),
            label: None,
            values: vec![
                ValueContent::Link {
                    target_iri: "http://rdfh.ch/0803/book1".into(),
                    target_label: Some("My Book".into()),
                }
                .into(),
                ValueContent::Link {
                    target_iri: "http://rdfh.ch/0803/book2".into(),
                    target_label: None,
                }
                .into(),
            ],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let vals = &parsed["data"]["values"][0]["values"];
        // first: with label
        assert_eq!(vals[0]["value_type"], "link");
        assert_eq!(vals[0]["target_iri"], "http://rdfh.ch/0803/book1");
        assert_eq!(vals[0]["target_label"], "My Book");
        // second: no label → null
        assert_eq!(vals[1]["value_type"], "link");
        assert_eq!(vals[1]["target_iri"], "http://rdfh.ch/0803/book2");
        assert!(vals[1]["target_label"].is_null());
    }

    #[test]
    fn resource_describe_json_still_image_value() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "hasStillImageFileValue".into(),
            label: None,
            values: vec![
                ValueContent::File(FileValue {
                    value_type: ValueType::StillImage,
                    filename: "image.jp2".into(),
                    url: "https://iiif.example.com/image.jp2/full/max/0/default.jpg".into(),
                    width: Some(1200),
                    height: Some(800),
                })
                .into(),
            ],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let val = &parsed["data"]["values"][0]["values"][0];
        assert_eq!(val["value_type"], "still-image");
        assert_eq!(val["filename"], "image.jp2");
        assert_eq!(val["url"], "https://iiif.example.com/image.jp2/full/max/0/default.jpg");
        assert_eq!(val["width"], 1200);
        assert_eq!(val["height"], 800);
    }

    #[test]
    fn resource_describe_json_date_value() {
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        let pt = DatePoint {
            year: Some(1489),
            month: None,
            day: None,
            era: Some("CE".into()),
        };
        detail.values = Some(vec![FieldValues {
            name: "hasDate".into(),
            label: None,
            values: vec![
                ValueContent::Date(DateValue {
                    calendar: "GREGORIAN".into(),
                    start: pt.clone(),
                    end: pt.clone(),
                })
                .into(),
            ],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let val = &parsed["data"]["values"][0]["values"][0];
        assert_eq!(val["value_type"], "date");
        assert_eq!(val["calendar"], "GREGORIAN");
        assert_eq!(val["start"]["year"], 1489);
        assert_eq!(val["start"]["era"], "CE");
        assert!(val["start"]["month"].is_null());
        assert!(val["start"]["day"].is_null());
    }

    #[test]
    fn resource_describe_json_comment_present_when_set() {
        // A value with a comment carries a "comment" key in the value object.
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "hasTranscription".into(),
            label: None,
            values: vec![Value {
                content: ValueContent::Text("some transcription".into()),
                comment: Some("reading uncertain".into()),
            }],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let val = &parsed["data"]["values"][0]["values"][0];
        assert_eq!(val["value_type"], "text");
        assert_eq!(val["text"], "some transcription");
        assert_eq!(val["comment"], "reading uncertain");
    }

    #[test]
    fn resource_describe_json_comment_absent_when_none() {
        // A value with no comment must NOT carry a "comment" key at all (omit, not null).
        let out = SharedBuf::new();
        let mut renderer = JsonRenderer::with_writer(out.clone());
        let mut detail = make_resource_detail_no_values();
        detail.values = Some(vec![FieldValues {
            name: "hasTranscription".into(),
            label: None,
            values: vec![ValueContent::Text("plain transcription".into()).into()],
        }]);
        let meta = make_meta("anonymous", "https://api.test.dasch.swiss");
        renderer.resource_describe(&detail, &meta).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(out.string().trim()).unwrap();
        let val = &parsed["data"]["values"][0]["values"][0];
        assert!(
            !val.as_object().unwrap().contains_key("comment"),
            "comment key must be absent when comment is None; got: {val}"
        );
    }
}
