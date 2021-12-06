//! Time-related functionality for Loiter.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::macros::time;
use time::{format_description, Date, Month, OffsetDateTime, PrimitiveDateTime, Time};

use crate::Error;

const DATE_TIME_FORMATS: &[&str] = &[
    "[year]-[month]-[day]T[hour]:[minute]",
    "[year]-[month]-[day] [hour]:[minute]",
    "[year]-[month]-[day]T[hour]:[minute]:[second]",
    "[year]-[month]-[day] [hour]:[minute]:[second]",
];

const DATE_TIME_FORMATS_WITH_OFFSET: &[&str] = &[
    "[year]-[month]-[day]T[hour]:[minute] [offset_hour sign:mandatory]:[offset_minute]",
    "[year]-[month]-[day] [hour]:[minute] [offset_hour sign:mandatory]:[offset_minute]",
    "[year]-[month]-[day]T[hour]:[minute]:[second] [offset_hour sign:mandatory]:[offset_minute]",
    "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour sign:mandatory]:[offset_minute]",
];

const TIME_ONLY_FORMATS: &[&str] = &[
    "[hour]:[minute]",
    "[hour]h[minute]",
    "[hour]:[minute]:[second]",
];

const DEFAULT_TIMESTAMP_FORMAT: &str =
    "[year]-[month]-[day] [hour]:[minute] [offset_hour sign:mandatory]";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Timestamp(#[serde(with = "timestamp_s18n")] OffsetDateTime);

impl Timestamp {
    /// Attempt to parse the given string as a timestamp, using `now` as a
    /// reference for the current time.
    pub fn parse(s: &str, now: Self) -> Result<Self, Error> {
        Ok(Self(parse_timestamp(s, &now.0)?))
    }

    /// Return the current date/time.
    ///
    /// Attempts to obtain time zone information from the system. Returns an
    /// error if this fails.
    pub fn now() -> Result<Self, Error> {
        Ok(Self(OffsetDateTime::now_local()?))
    }

    /// Return the timestamp of the beginning of the day today.
    pub fn today(&self) -> Self {
        Self(self.0.replace_time(time!(00:00)))
    }

    /// Timestamp as at the beginning of tomorrow.
    pub fn tomorrow(&self) -> Self {
        Self(self.today().0 + time::Duration::DAY)
    }

    /// Timestamp as at the beginning of yesterday.
    pub fn yesterday(&self) -> Self {
        Self(self.today().0 - time::Duration::DAY)
    }

    /// Return the timestamp of the beginning of the day on Monday of this week.
    pub fn this_week(&self) -> Self {
        let today = self.today().0;
        let days_from_monday = today.weekday().number_days_from_monday();
        // Subtract the number of days from Monday
        let monday = today
            - time::Duration::DAY
                .checked_mul(days_from_monday.into())
                .unwrap();
        Self(monday)
    }

    /// Timestamp 1 week from the beginning of this week.
    pub fn next_week(&self) -> Self {
        Self(self.this_week().0 + time::Duration::DAY.checked_mul(7).unwrap())
    }

    /// Return the timestamp of the beginning of the day of the given number of
    /// days back in time.
    pub fn days_back(&self, days: u16) -> Self {
        Self(self.today().0 - time::Duration::DAY.checked_mul(days.into()).unwrap())
    }

    /// Timestamp at the beginning of the day of the given number of days
    /// forward in time.
    pub fn days_forward(&self, days: u16) -> Self {
        Self(self.today().0 + time::Duration::DAY.checked_mul(days.into()).unwrap())
    }

    /// Return the timestamp of the beginning of the day on the first day of
    /// this month.
    pub fn this_month(&self) -> Self {
        let today = self.today().0;
        Self(today.replace_date(Date::from_calendar_date(today.year(), today.month(), 1).unwrap()))
    }

    /// Timestamp as at the beginning of the day on the first day of next month.
    pub fn next_month(&self) -> Self {
        let next_month = self.this_month().0 + (32 * time::Duration::DAY);
        Self(next_month.replace_date(
            Date::from_calendar_date(next_month.year(), next_month.month(), 1).unwrap(),
        ))
    }

    /// Return the timestamp of the beginning of the day on the first of January
    /// of this year.
    pub fn this_year(&self) -> Self {
        let today = self.today().0;
        Self(today.replace_date(Date::from_calendar_date(today.year(), Month::January, 1).unwrap()))
    }

    /// Timestamp as at the beginning of the day on the first of January of next
    /// year.
    pub fn next_year(&self) -> Self {
        let next_year = self.this_year().0 + (366 * time::Duration::DAY);
        Self(
            next_year.replace_date(
                Date::from_calendar_date(next_year.year(), Month::January, 1).unwrap(),
            ),
        )
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self(OffsetDateTime::now_utc())
    }
}

impl From<OffsetDateTime> for Timestamp {
    fn from(dt: OffsetDateTime) -> Self {
        Self(dt)
    }
}

impl From<Timestamp> for OffsetDateTime {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

impl FromStr for Timestamp {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Timestamp(parse_timestamp(
            s,
            &OffsetDateTime::now_local()?,
        )?))
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .format(&format_description::parse(DEFAULT_TIMESTAMP_FORMAT).unwrap())
                .unwrap()
        )
    }
}

impl std::ops::Sub for Timestamp {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        Duration(self.0 - rhs.0)
    }
}

mod timestamp_s18n {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};

    pub fn serialize<S>(value: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.format(&Rfc3339).map_err(serde::ser::Error::custom)?)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        OffsetDateTime::parse(&s, &Rfc3339).map_err(serde::de::Error::custom)
    }
}

// Attempts to parse the given timestamp in a somewhat human-friendly way.
fn parse_timestamp<S: AsRef<str>>(
    ts: S,
    local_now: &OffsetDateTime,
) -> Result<OffsetDateTime, Error> {
    let ts_orig = ts.as_ref().to_string();

    if ts_orig.trim() == "now" {
        return Ok(*local_now);
    }

    for fmt in DATE_TIME_FORMATS {
        if let Ok(dt) = PrimitiveDateTime::parse(&ts_orig, &format_description::parse(fmt)?) {
            let dt = dt.assume_offset(local_now.offset());
            return Ok(dt);
        }
    }
    for fmt in DATE_TIME_FORMATS_WITH_OFFSET {
        if let Ok(dt) = OffsetDateTime::parse(&ts_orig, &format_description::parse(fmt)?) {
            return Ok(dt);
        }
    }

    let (prefix_offset, ts) = parse_prefix_offset(&ts_orig)?;
    for fmt in TIME_ONLY_FORMATS {
        if let Ok(t) = Time::parse(&ts, &format_description::parse(fmt)?) {
            let dt = local_now.replace_time(t) + prefix_offset;
            return Ok(dt);
        }
    }

    Err(Error::InvalidDateTime(ts_orig))
}

fn parse_prefix_offset(ts: &str) -> Result<(time::Duration, String), Error> {
    let ts = ts.to_string();
    let ts_lower = ts.to_lowercase();
    let ts_parts = ts_lower
        .split('@')
        .map(|p| p.to_string())
        .collect::<Vec<String>>();
    let (maybe_prefix, ts_suffix) = if ts_parts.len() == 2 {
        (
            Some(ts_parts[0].trim().to_string()),
            ts_parts[1].trim().to_string(),
        )
    } else if ts_parts.len() == 1 {
        (None, ts_parts[0].trim().to_string())
    } else {
        return Err(Error::InvalidDateTime(ts));
    };
    Ok((
        maybe_prefix
            .map(|prefix| {
                Ok(match prefix.as_str() {
                    "yesterday" | "yst" => -time::Duration::DAY,
                    "tomorrow" | "tmrw" => time::Duration::DAY,
                    _ => return Err(Error::InvalidDateTime(ts)),
                })
            })
            .unwrap_or(Ok(time::Duration::ZERO))?,
        ts_suffix,
    ))
}

/// Provides parsing of somewhat human-friendly durations.
///
/// Examples:
///
/// - `1h` is parsed to 1 hour
/// - `30m` is parsed to 30 minutes
/// - `1h30m` is parsed to 1 hour and 30 minutes
/// - `1d` is parsed to 1 day
/// - `1w` is parsed to 1 week
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(time::Duration);

impl Duration {
    pub fn zero() -> Self {
        Self(time::Duration::ZERO)
    }
}

impl From<time::Duration> for Duration {
    fn from(d: time::Duration) -> Self {
        Self(d)
    }
}

impl From<Duration> for time::Duration {
    fn from(d: Duration) -> Self {
        d.0
    }
}

impl FromStr for Duration {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let components = parse_duration_components(s)?;
        Ok(Self(
            components
                .into_iter()
                .map(|(amt_str, unit_str)| {
                    let amt = i32::from_str(&amt_str)
                        .map_err(|e| Error::InvalidDurationAmount(amt_str.clone(), e))?;
                    let unit = match unit_str.as_str() {
                        "w" => time::Duration::WEEK,
                        "d" => time::Duration::DAY,
                        "h" => time::Duration::HOUR,
                        "m" => time::Duration::MINUTE,
                        "s" => time::Duration::SECOND,
                        _ => return Err(Error::InvalidDurationUnit(unit_str)),
                    };
                    let duration = unit
                        .checked_mul(amt)
                        .ok_or_else(|| Error::InvalidDuration(amt_str.clone(), unit_str))?;
                    Ok(duration)
                })
                .collect::<Result<Vec<time::Duration>, Self::Err>>()?
                .into_iter()
                .sum(),
        ))
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let weeks = self.0.whole_weeks();
        let weeks_duration = time::Duration::WEEK
            .checked_mul(weeks.try_into().unwrap())
            .unwrap();
        let days = (self.0 - weeks_duration).whole_days();
        let days_duration = time::Duration::DAY
            .checked_mul(days.try_into().unwrap())
            .unwrap();
        let hours = (self.0 - weeks_duration - days_duration).whole_hours();
        let hours_duration = time::Duration::HOUR
            .checked_mul(hours.try_into().unwrap())
            .unwrap();
        let mins = (self.0 - weeks_duration - days_duration - hours_duration).whole_minutes();
        let mins_duration = time::Duration::MINUTE
            .checked_mul(mins.try_into().unwrap())
            .unwrap();
        let secs = (self.0 - weeks_duration - days_duration - hours_duration - mins_duration)
            .whole_seconds();

        let s: String = [
            (weeks, "w"),
            (days, "d"),
            (hours, "h"),
            (mins, "m"),
            (secs, "s"),
        ]
        .into_iter()
        .filter_map(|(amt, unit)| {
            if amt > 0 {
                Some(format!("{}{}", amt, unit))
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join(" ");
        write!(f, "{}", s)
    }
}

impl Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Duration::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl std::ops::Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

enum DurationParserState {
    Begin,
    Amount,
    Unit,
}

fn parse_duration_components<S: AsRef<str>>(s: S) -> Result<Vec<(String, String)>, Error> {
    let s = s.as_ref().to_string();
    let mut result = Vec::new();
    let mut state = DurationParserState::Begin;
    let mut cur_amount = String::new();
    let mut cur_unit = String::new();

    for c in s.chars() {
        match state {
            DurationParserState::Begin => {
                if !c.is_digit(10) {
                    return Err(Error::DurationMustStartWithNumber(s.clone()));
                }
                cur_amount.push(c);
                state = DurationParserState::Amount;
            }
            DurationParserState::Amount => {
                if c.is_digit(10) {
                    cur_amount.push(c);
                } else {
                    state = DurationParserState::Unit;
                    cur_unit.push(c);
                }
            }
            DurationParserState::Unit => {
                if c.is_digit(10) {
                    result.push((cur_amount.trim().to_string(), cur_unit.trim().to_string()));
                    cur_amount.truncate(0);
                    cur_unit.truncate(0);
                    cur_amount.push(c);
                    state = DurationParserState::Amount;
                } else {
                    cur_unit.push(c);
                }
            }
        }
    }

    if !cur_amount.is_empty() && !cur_unit.is_empty() {
        result.push((cur_amount, cur_unit));
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::{parse_timestamp, Duration};
    use lazy_static::lazy_static;
    use std::str::FromStr;
    use time::macros::datetime;
    use time::OffsetDateTime;

    lazy_static! {
        static ref LOCAL_NOW: OffsetDateTime = datetime!(2021-11-04 17:00 -4);
        static ref TIMESTAMP_TEST_CASES: Vec<(String, OffsetDateTime)> = vec![
            (
                "2021-11-04 12:43".to_string(),
                datetime!(2021-11-04 12:43 -4)
            ),
            (
                "2021-11-04 12:43:23".to_string(),
                datetime!(2021-11-04 12:43:23 -4),
            ),
            (
                "2021-11-04 12:43:23 +02:00".to_string(),
                datetime!(2021-11-04 12:43:23 +2),
            ),
            (
                "yesterday@10:00".to_string(),
                datetime!(2021-11-03 10:00 -4),
            ),
            ("yst@10:00".to_string(), datetime!(2021-11-03 10:00 -4),),
            ("tomorrow@10:00".to_string(), datetime!(2021-11-05 10:00 -4),),
            ("tmrw@10:00".to_string(), datetime!(2021-11-05 10:00 -4),),
            ("10:00".to_string(), datetime!(2021-11-04 10:00 -4),),
            ("10:23:44".to_string(), datetime!(2021-11-04 10:23:44 -4),),
            ("now".to_string(), datetime!(2021-11-04 17:00 -4)),
        ];
        static ref DURATION_PARSE_TEST_CASES: Vec<(String, i64)> = vec![
            ("1m".to_string(), 60),
            ("2m".to_string(), 2 * 60),
            ("80m".to_string(), 80 * 60),
            ("1h".to_string(), 60 * 60),
            ("1h30m".to_string(), (60 * 60) + (30 * 60)),
            ("1d".to_string(), 24 * 60 * 60),
            (
                "1d4h12m".to_string(),
                (24 * 60 * 60) + (4 * 60 * 60) + (12 * 60)
            ),
        ];
        static ref DURATION_FORMAT_TEST_CASES: Vec<(i64, String)> = vec![
            (60, "1m".to_string()),
            (1, "1s".to_string()),
            (60 * 60, "1h".to_string()),
            ((30 * 60) + (60 * 60), "1h 30m".to_string()),
            (24 * 60 * 60, "1d".to_string()),
            (7 * 24 * 60 * 60, "1w".to_string()),
            (
                (7 * 24 * 60 * 60) + (3 * 24 * 60 * 60) + (4 * 60 * 60),
                "1w 3d 4h".to_string()
            ),
        ];
    }

    #[test]
    fn timestamp_parsing() {
        for (ts, expected) in TIMESTAMP_TEST_CASES.iter() {
            let actual = parse_timestamp(ts, &LOCAL_NOW).unwrap();
            assert_eq!(&actual, expected);
        }
    }

    #[test]
    fn duration_parsing() {
        for (s, expected) in DURATION_PARSE_TEST_CASES.iter() {
            let actual: time::Duration = Duration::from_str(s).unwrap().into();
            assert_eq!(actual.whole_seconds(), *expected);
        }
    }

    #[test]
    fn duration_formatting() {
        for (secs, expected) in DURATION_FORMAT_TEST_CASES.iter() {
            let duration: Duration = time::Duration::seconds(*secs).into();
            let actual = duration.to_string();
            assert_eq!(&actual, expected);
        }
    }
}
