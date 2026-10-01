//! A date value's deviation triples (`FORMAT.md` §4.6).

use cpe_ports::{Calendar, DateBound, DatePrecision, DateValue};
use oxrdf::vocab::xsd;
use oxrdf::{NamedNodeRef, TermRef};

use super::{integer, invalid, lexical, one, Facts, Invalid};
use crate::vocab::{
    DAO_DATE_CALENDAR, DAO_DATE_END_JDN, DAO_DATE_END_PRECISION, DAO_DATE_START_JDN, DAO_DATE_START_PRECISION,
};
use crate::InvalidFact;

pub(super) fn map(node: &str, facts: &Facts<'_>) -> Result<DateValue, Invalid> {
    let calendar = match term(node, facts, DAO_DATE_CALENDAR)? {
        "GREGORIAN" => Calendar::Gregorian,
        "JULIAN" => Calendar::Julian,
        other => return Err(unknown(node, DAO_DATE_CALENDAR, other)),
    };
    let start_jdn = integer(node, DAO_DATE_START_JDN, required(node, facts, DAO_DATE_START_JDN)?)?;
    let end_jdn = integer(node, DAO_DATE_END_JDN, required(node, facts, DAO_DATE_END_JDN)?)?;
    Ok(DateValue {
        calendar,
        start: DateBound {
            jdn: start_jdn,
            precision: precision(node, facts, DAO_DATE_START_PRECISION)?,
        },
        end: DateBound {
            jdn: end_jdn,
            precision: precision(node, facts, DAO_DATE_END_PRECISION)?,
        },
    })
}

fn precision(node: &str, facts: &Facts<'_>, predicate: NamedNodeRef<'static>) -> Result<DatePrecision, Invalid> {
    match term(node, facts, predicate)? {
        "YEAR" => Ok(DatePrecision::Year),
        "MONTH" => Ok(DatePrecision::Month),
        "DAY" => Ok(DatePrecision::Day),
        other => Err(unknown(node, predicate, other)),
    }
}

/// The lexical form of the plain literal a date term is written as; any other term is an unknown
/// one.
fn term<'a>(node: &str, facts: &Facts<'a>, predicate: NamedNodeRef<'static>) -> Result<&'a str, Invalid> {
    match required(node, facts, predicate)? {
        TermRef::Literal(literal) if literal.datatype() == xsd::STRING => Ok(literal.value()),
        other => Err(unknown(node, predicate, &lexical(other))),
    }
}

fn required<'a>(node: &str, facts: &Facts<'a>, predicate: NamedNodeRef<'static>) -> Result<TermRef<'a>, Invalid> {
    one(node, facts, predicate)?
        .ok_or_else(|| invalid(node, InvalidFact::IncompleteDate { missing: predicate.as_str().to_string() }))
}

fn unknown(node: &str, predicate: NamedNodeRef<'_>, lexical: &str) -> Invalid {
    invalid(
        node,
        InvalidFact::UnknownDateTerm {
            predicate: predicate.as_str().to_string(),
            lexical: lexical.to_string(),
        },
    )
}
