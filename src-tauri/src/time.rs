//! Dates from the API (ISO 8601 / RFC 3339) as Unix milliseconds, without
//! a date crate.

/// Reads an ISO 8601 / RFC 3339 date-time (`2026-10-18T12:34:56.789Z`,
/// `...-03:00`) as Unix milliseconds. Only the first 3 fraction digits count.
pub fn parse_timestamp_ms(text: &str) -> Option<i64> {
    let (date, time) = text.trim().split_once(['T', 't', ' '])?;

    let mut parts = date.splitn(3, '-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;

    let (clock, offset_secs) = match time.strip_suffix(['Z', 'z']) {
        Some(clock) => (clock, 0),
        None => {
            let at = time.rfind(['+', '-'])?;
            let (clock, zone) = time.split_at(at);
            let (hours, minutes) = zone[1..].split_once(':')?;
            let secs = hours.parse::<i64>().ok()? * 3600 + minutes.parse::<i64>().ok()? * 60;
            (clock, if zone.starts_with('-') { -secs } else { secs })
        }
    };

    let (hms, fraction) = clock.split_once('.').unwrap_or((clock, ""));
    let mut parts = hms.splitn(3, ':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    let second: i64 = parts.next()?.parse().ok()?;
    if !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let millis: i64 = format!("{fraction:0<3}")[..3].parse().ok()?;

    let valid = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second <= 60;
    if !valid {
        return None;
    }

    let secs = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    Some((secs - offset_secs) * 1000 + millis)
}

/// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Year, month and day of a count of days since 1970-01-01 (Howard
/// Hinnant's `civil_from_days`, the inverse of `days_from_civil`).
pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_from_days_inverts_days_from_civil() {
        for (y, m, d) in [(1970, 1, 1), (2000, 2, 29), (2026, 10, 18), (1969, 12, 31), (2100, 3, 1)] {
            assert_eq!(civil_from_days(days_from_civil(y, m, d)), (y, m, d));
        }
    }

    #[test]
    fn parses_utc_dates_with_milliseconds() {
        assert_eq!(parse_timestamp_ms("2026-10-18T12:34:56.789Z"), Some(1_792_326_896_789));
        assert_eq!(parse_timestamp_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_timestamp_ms("1969-12-31T23:59:59Z"), Some(-1000));
    }

    #[test]
    fn parses_offsets_and_long_fractions() {
        assert_eq!(parse_timestamp_ms("2026-10-18T09:34:56-03:00"), Some(1_792_326_896_000));
        // Go writes up to 9 fraction digits.
        assert_eq!(parse_timestamp_ms("2026-10-18T12:34:56.789123456Z"), Some(1_792_326_896_789));
        assert_eq!(parse_timestamp_ms("2026-10-18T12:34:56.5Z"), Some(1_792_326_896_500));
    }

    #[test]
    fn rejects_text_that_is_not_a_date() {
        for text in ["", "yesterday", "2026-10-18", "2026-13-01T00:00:00Z", "2026-10-18T12:34:56.x1Z"] {
            assert_eq!(parse_timestamp_ms(text), None, "{text}");
        }
    }
}
