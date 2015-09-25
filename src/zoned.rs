//! Dates and times paired with a time zone, and time zone definitions.

use local::{LocalDateTime, LocalDate, LocalTime, DatePiece, TimePiece, Month, Weekday};
use local::ParseError as LocalParseError;
use parse;
use util::RangeExt;

use std::error::Error as StdError;
use std::fs::File;
use std::io::Read;
use std::num::ParseIntError;
use std::path::Path;
use std::str::FromStr;

use duration::Duration;
use tz::{Transition, parse};


/// A **time zone** is used to calculate how much to adjust a UTC-based time
/// based on its geographical location.
#[derive(Clone, Debug)]
pub enum TimeZone {
    UTC,
    FixedOffset { offset: i32 },
    VariableOffset { transitions: Vec<Transition> }
}

/// A **time zone** is used to calculate how much to adjust a UTC-based time
/// based on its geographical location.
impl TimeZone {
    fn adjust(&self, local: LocalDateTime) -> LocalDateTime {
        match *self {
            TimeZone::UTC                                 => { self.adjust_utc(local) },
            TimeZone::FixedOffset { offset }              => { self.adjust_fixed(offset, local) },
            TimeZone::VariableOffset { ref transitions }  => { self.adjust_variable(&transitions, local) },
        }
    }

    fn adjust_utc(&self, local: LocalDateTime) -> LocalDateTime {
        local  // No adjustment needed! LocalDateTime uses UTC.
    }

    fn adjust_fixed(&self, offset: i32,  local: LocalDateTime) -> LocalDateTime {
        local + Duration::of(offset as i64)
    }

    fn adjust_variable(&self, transitions: &Vec<Transition>, local: LocalDateTime) -> LocalDateTime {
        let unix_timestamp = local.to_instant().seconds() as i32;

        // TODO: Replace this with a binary search
        match transitions.iter().find(|t| t.timestamp < unix_timestamp) {
            None     => local,
            Some(t)  => local + Duration::of(t.local_time_type.offset as i64),
        }
    }

    pub fn at(&self, local: LocalDateTime) -> ZonedDateTime {
        ZonedDateTime {
            local: local,
            time_zone: self.clone()
        }
    }

    /// Read time zone information in from the user's local time zone.
    pub fn localtime() -> Result<TimeZone, Box<StdError>> {
        // TODO: replace this with some kind of factory.
        // this won't be appropriate for all systems
        TimeZone::zoneinfo(&Path::new("/etc/localtime"))
    }

    /// Read time zone information in from the file at the given path,
    /// returning a variable offset containing time transitions if successful,
    /// or an error if not.
    pub fn zoneinfo(path: &Path) -> Result<TimeZone, Box<StdError>> {
        let mut contents = Vec::new();
        let mut file     = try!(File::open(path));
        let _bytes_read  = try!(file.read_to_end(&mut contents));
        let mut tz       = try!(parse(contents));

        // Sort the transitions *backwards* to make it easier to get the first
        // one *after* a specified time.
        tz.transitions.sort_by(|b, a| a.timestamp.cmp(&b.timestamp));

        Ok(TimeZone::VariableOffset { transitions: tz.transitions })
    }

    /// Create a new fixed-offset timezone with the given number of seconds.
    ///
    /// Returns an error if the number of seconds is greater than one day's
    /// worth of seconds (86400) in either direction.
    pub fn of_seconds(seconds: i32) -> Result<TimeZone, Error> {
        if seconds.is_within(-86400..86401) {
            Ok(TimeZone::FixedOffset { offset: seconds })
        }
        else {
            Err(Error::OutOfRange)
        }
    }

    /// Create a new fixed-offset timezone with the given number of hours and
    /// minutes.
    ///
    /// The values should either be both positive or both negative.
    ///
    /// Returns an error if the numbers are greater than their unit allows
    /// (more than 23 hours or 59 minutes) in either direction, or if the
    /// values differ in sign (such as a positive number of hours with a
    /// negative number of minutes).
    pub fn of_hours_and_minutes(hours: i8, minutes: i8) -> Result<TimeZone, Error> {
        if (hours.is_positive() && minutes.is_negative())
        || (hours.is_negative() && minutes.is_positive()) {
            Err(Error::SignMismatch)
        }
        else if hours <= -24 || hours >= 24 {
            Err(Error::OutOfRange)
        }
        else if minutes <= -60 || minutes >= 60 {
            Err(Error::OutOfRange)
        }
        else {
            let hours = hours as i32;
            let minutes = minutes as i32;
            TimeZone::of_seconds(hours * 24 + minutes * 60)
        }
    }

    pub fn from_fields(fields: parse::ZoneFields) -> Result<TimeZone, ParseError> {
        use parse::ZoneFields::*;
        let parse = |input: &str| input.parse().map_err(ParseError::Number);

        let result = match fields {
            Zulu => return Ok(TimeZone::UTC),
            Offset { sign: "+", hours, minutes: None } => TimeZone::of_hours_and_minutes( try!(parse(hours)), 0),
            Offset { sign: "-", hours, minutes: None } => TimeZone::of_hours_and_minutes(-try!(parse(hours)), 0),
            Offset { sign: "+", hours, minutes: Some(mins) } => TimeZone::of_hours_and_minutes( try!(parse(hours)),  try!(parse(mins))),
            Offset { sign: "-", hours, minutes: Some(mins) } => TimeZone::of_hours_and_minutes(-try!(parse(hours)), -try!(parse(mins))),
            _ => unreachable!(),  // this definitely should be unreachable: the regex only checks for [Z+-].
        };

        result.map_err(ParseError::Zone)
    }
}

impl FromStr for TimeZone {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<TimeZone, Self::Err> {
        match parse::parse_iso_8601_zone(input) {
            Ok(fields)  => TimeZone::from_fields(fields),
            Err(e)      => Err(ParseError::Parse(e)),
        }
    }
}


#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Error {
    OutOfRange,
    SignMismatch,
}

#[derive(PartialEq, Debug, Clone)]
pub enum ParseError {
    Zone(Error),
    Date(LocalParseError),
    Number(ParseIntError),
    Parse(parse::Error),
}



/// A time paired with a time zone.
#[derive(Debug, Clone)]
pub struct ZonedDateTime {
    local: LocalDateTime,
    time_zone: TimeZone,
}

impl FromStr for ZonedDateTime {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<ZonedDateTime, Self::Err> {
        let (date_fields, time_fields, zone_fields) = try!(parse::parse_iso_8601_date_time_zone(input).map_err(ParseError::Parse));
        let date = try!(LocalDate::from_fields(date_fields).map_err(ParseError::Date));
        let time = try!(LocalTime::from_fields(time_fields).map_err(ParseError::Date));
        let zone = try!(TimeZone::from_fields(zone_fields));
        Ok(ZonedDateTime { local: LocalDateTime::new(date, time), time_zone: zone })
    }
}


impl DatePiece for ZonedDateTime {
    fn year(&self) -> i64 {
        self.time_zone.adjust(self.local).year()
    }

    fn month(&self) -> Month {
        self.time_zone.adjust(self.local).month()
    }

    fn day(&self) -> i8 {
        self.time_zone.adjust(self.local).day()
    }

    fn yearday(&self) -> i16 {
        self.time_zone.adjust(self.local).yearday()
    }

    fn weekday(&self) -> Weekday {
        self.time_zone.adjust(self.local).weekday()
    }
}

impl TimePiece for ZonedDateTime {
    fn hour(&self) -> i8 {
        self.time_zone.adjust(self.local).hour()
    }

    fn minute(&self) -> i8 {
        self.time_zone.adjust(self.local).minute()
    }

    fn second(&self) -> i8 {
        self.time_zone.adjust(self.local).second()
    }

    fn millisecond(&self) -> i16 {
        self.time_zone.adjust(self.local).millisecond()
    }
}


#[cfg(test)]
mod test {
    use super::TimeZone;

    #[test]
    fn fixed_seconds() {
        assert!(TimeZone::of_seconds(1234).is_ok());
    }

    #[test]
    fn fixed_seconds_panic() {
        assert!(TimeZone::of_seconds(100_000).is_err());
    }

    #[test]
    fn fixed_hm() {
        assert!(TimeZone::of_hours_and_minutes(5, 30).is_ok());
    }

    #[test]
    fn fixed_hm_negative() {
        assert!(TimeZone::of_hours_and_minutes(-3, -45).is_ok());
    }

    #[test]
    fn fixed_hm_err() {
        assert!(TimeZone::of_hours_and_minutes(8, 60).is_err());
    }

    #[test]
    fn fixed_hm_signs() {
        assert!(TimeZone::of_hours_and_minutes(-4, 30).is_err());
    }

    #[test]
    fn fixed_hm_signs_zero() {
        assert!(TimeZone::of_hours_and_minutes(4, 0).is_ok());
    }
}
