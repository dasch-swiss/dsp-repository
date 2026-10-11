//! A Julian Day Number as a date of the Julian or the Gregorian calendar, and its text at a
//! precision.
//!
//! The conversion is Richards' algorithm (Explanatory Supplement to the Astronomical Almanac,
//! 3rd edition, 15.11.3).

use cpe_ports::{Calendar, DatePrecision};

/// A day of a calendar, in the years 1 to 9999.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Day {
    year: i64,
    month: i64,
    day: i64,
}

/// 1 January of year 1 in the Julian calendar, the earlier of the two.
const FIRST_JDN: i64 = 1_721_424;
/// 31 December 9999 in the Julian calendar, the later of the two.
const LAST_JDN: i64 = 5_373_557;

/// The day `jdn` is in `calendar`, or `None` for a day outside the years 1 to 9999: the store
/// writes a year as four digits and has no form for a year before 1.
///
/// The bounds are checked before the arithmetic, which would overflow for a far-off number.
pub(crate) fn day(jdn: i64, calendar: Calendar) -> Option<Day> {
    if !(FIRST_JDN..=LAST_JDN).contains(&jdn) {
        return None;
    }
    let f = jdn
        + 1401
        + match calendar {
            Calendar::Julian => 0,
            Calendar::Gregorian => ((4 * jdn + 274_277).div_euclid(146_097) * 3).div_euclid(4) - 38,
        };
    let e = 4 * f + 3;
    let g = e.rem_euclid(1461).div_euclid(4);
    let h = 5 * g + 2;
    let day = h.rem_euclid(153).div_euclid(5) + 1;
    let month = (h.div_euclid(153) + 2).rem_euclid(12) + 1;
    let year = e.div_euclid(1461) - 4716 + (12 + 2 - month).div_euclid(12);
    (1..=9999).contains(&year).then_some(Day { year, month, day })
}

impl Day {
    /// The store's text for the day at `precision`: `YYYY`, `YYYY-MM` or `YYYY-MM-DD`.
    pub(crate) fn text(self, precision: DatePrecision) -> String {
        let Day { year, month, day } = self;
        match precision {
            DatePrecision::Year => format!("{year:04}"),
            DatePrecision::Month => format!("{year:04}-{month:02}"),
            DatePrecision::Day => format!("{year:04}-{month:02}-{day:02}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(jdn: i64, calendar: Calendar, precision: DatePrecision) -> Option<String> {
        day(jdn, calendar).map(|day| day.text(precision))
    }

    #[test]
    fn test_day_known_days_of_both_calendars() {
        let cases = [
            (2_299_161, Calendar::Gregorian, "1582-10-15"),
            (2_299_161, Calendar::Julian, "1582-10-05"),
            (2_299_160, Calendar::Julian, "1582-10-04"),
            (2_451_545, Calendar::Gregorian, "2000-01-01"),
            (2_440_588, Calendar::Gregorian, "1970-01-01"),
            (1_721_426, Calendar::Gregorian, "0001-01-01"),
            (1_721_424, Calendar::Julian, "0001-01-01"),
            (2_266_011, Calendar::Julian, "1492-01-01"),
            (2_266_376, Calendar::Julian, "1492-12-31"),
            (2_265_695, Calendar::Julian, "1491-02-19"),
            (5_373_484, Calendar::Gregorian, "9999-12-31"),
            (5_373_557, Calendar::Julian, "9999-12-31"),
        ];

        for (jdn, calendar, expected) in cases {
            assert_eq!(
                text(jdn, calendar, DatePrecision::Day).as_deref(),
                Some(expected),
                "{jdn} {calendar:?}"
            );
        }
    }

    #[test]
    fn test_text_is_cut_to_the_precision() {
        let february = 2_265_695;

        assert_eq!(
            text(february, Calendar::Julian, DatePrecision::Month).as_deref(),
            Some("1491-02")
        );
        assert_eq!(text(february, Calendar::Julian, DatePrecision::Year).as_deref(), Some("1491"));
        assert_eq!(text(1_721_424, Calendar::Julian, DatePrecision::Year).as_deref(), Some("0001"));
    }

    #[test]
    fn test_day_outside_the_years_1_to_9999_is_none() {
        // The last day of the year before 1, and the first of 10000, in each calendar.
        assert_eq!(day(1_721_425, Calendar::Gregorian), None);
        assert_eq!(day(1_721_423, Calendar::Julian), None);
        assert_eq!(day(5_373_485, Calendar::Gregorian), None);
        assert_eq!(day(5_373_558, Calendar::Julian), None);
    }

    #[test]
    fn test_day_far_off_number_is_none_without_overflow() {
        for jdn in [i64::MIN, -1, 0, i64::MAX] {
            assert_eq!(day(jdn, Calendar::Gregorian), None, "{jdn}");
            assert_eq!(day(jdn, Calendar::Julian), None, "{jdn}");
        }
    }
}
