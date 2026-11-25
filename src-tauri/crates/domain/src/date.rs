//! Reading the game's stored dates.
//!
//! Every date the game stores is `YYYY-MM-DD` text, and the day loop reads tens of
//! thousands of them a day (birthdays, fixtures, contracts). Chrono's format parser
//! handles any pattern and costs accordingly; this reads the one shape the game writes
//! directly and leaves anything else to chrono, so it accepts exactly what
//! `NaiveDate::parse_from_str(text, "%Y-%m-%d")` accepts.

use chrono::NaiveDate;

/// A stored date, or `None` when the text is not a date.
pub fn parse_iso_date(text: &str) -> Option<NaiveDate> {
    match text.as_bytes() {
        [y1, y2, y3, y4, b'-', m1, m2, b'-', d1, d2]
            if [y1, y2, y3, y4, m1, m2, d1, d2]
                .iter()
                .all(|byte| byte.is_ascii_digit()) =>
        {
            let digit = |byte: &u8| u32::from(byte - b'0');
            let year = digit(y1) * 1000 + digit(y2) * 100 + digit(y3) * 10 + digit(y4);
            let month = digit(m1) * 10 + digit(m2);
            let day = digit(d1) * 10 + digit(d2);
            NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, month, day)
        }
        _ => NaiveDate::parse_from_str(text, "%Y-%m-%d").ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chrono(text: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
    }

    /// Given the dates the game writes, when they are read, then each is the date chrono reads.
    #[test]
    fn a_stored_date_reads_as_chrono_reads_it() {
        for text in [
            "2033-07-01",
            "2000-02-29",
            "1999-12-31",
            "0001-01-01",
            "2026-11-24",
        ] {
            assert_eq!(parse_iso_date(text), chrono(text), "{text}");
            assert!(parse_iso_date(text).is_some(), "{text}");
        }
    }

    /// Given text that is not a real date, or not one at all, when it is read, then it is
    /// refused exactly where chrono refuses it.
    #[test]
    fn what_is_not_a_date_is_refused_as_chrono_refuses_it() {
        for text in [
            "2001-02-29",
            "2033-13-01",
            "2033-00-10",
            "2033-07-32",
            "2033-07-00",
            "",
            "unknown",
            "2033/07/01",
            "2033-07-01 ",
            " 2033-07-01",
            "2033-7-1",
            "33-07-01",
            "+2033-07-01",
            "2033-07-0a",
        ] {
            assert_eq!(parse_iso_date(text), chrono(text), "{text:?}");
        }
    }
}
