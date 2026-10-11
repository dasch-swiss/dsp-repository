//! The rows of `properties`, one constructor per value type. A row holds its type's columns and
//! leaves every other column NULL.

use cpe_ports::{Calendar, DatePrecision};

use crate::store::PropertyRow;

fn row(resource_id: &str, name: &str, seq: i64, value_type: &str) -> PropertyRow {
    PropertyRow {
        resource_id: resource_id.to_string(),
        name: name.to_string(),
        seq,
        value_type: value_type.to_string(),
        value_text: None,
        value_lang: None,
        value_int: None,
        value_real: None,
        date_start: None,
        date_end: None,
        date_precision: None,
        date_display: None,
        date_calendar: None,
        target_id: None,
    }
}

/// One `text` row, numbered 0.
pub(crate) fn text(resource_id: &str, name: &str, text: &str, lang: Option<&str>) -> PropertyRow {
    PropertyRow {
        value_text: Some(text.to_string()),
        value_lang: lang.map(str::to_string),
        ..row(resource_id, name, 0, "text")
    }
}

pub(crate) fn integer(resource_id: &str, name: &str, value: i64) -> PropertyRow {
    PropertyRow {
        value_int: Some(value),
        ..row(resource_id, name, 0, "integer")
    }
}

/// A link to the resource `target_id`.
pub(crate) fn link(resource_id: &str, name: &str, target_id: &str) -> PropertyRow {
    PropertyRow {
        target_id: Some(target_id.to_string()),
        ..row(resource_id, name, 0, "link")
    }
}

/// A value of a list: `node_id` is the node's id in `list_nodes`.
pub(crate) fn list_value(resource_id: &str, name: &str, node_id: &str) -> PropertyRow {
    PropertyRow {
        target_id: Some(node_id.to_string()),
        ..row(resource_id, name, 0, "list_value")
    }
}

/// A date from `start` to `end`, the two `bounds`, both written at `precision`. `display` is the
/// text a reader is shown for it and that text's language, where the date has one.
///
/// The words for a precision and a calendar are the ones the schema's checks on
/// `date_precision` and `date_calendar` admit.
pub(crate) fn date(
    resource_id: &str,
    name: &str,
    (start, end): (String, String),
    precision: DatePrecision,
    calendar: Calendar,
    display: Option<(&str, &str)>,
) -> PropertyRow {
    let precision = match precision {
        DatePrecision::Year => "year",
        DatePrecision::Month => "month",
        DatePrecision::Day => "day",
    };
    let calendar = match calendar {
        Calendar::Julian => "julian",
        Calendar::Gregorian => "gregorian",
    };
    PropertyRow {
        value_lang: display.map(|(_, lang)| lang.to_string()),
        date_start: Some(start),
        date_end: Some(end),
        date_precision: Some(precision.to_string()),
        date_display: display.map(|(text, _)| text.to_string()),
        date_calendar: Some(calendar.to_string()),
        ..row(resource_id, name, 0, "date")
    }
}
