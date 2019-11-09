#![crate_name = "datetime"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]

#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
//#![warn(missing_docs)]

#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(unused_results)]
#![allow(clippy::trivially_copy_pass_by_ref, clippy::missing_safety_doc)]

extern crate locale;
extern crate libc;
extern crate num_traits;
extern crate pad;
extern crate iso8601;

#[cfg(windows)] extern crate kernel32;
#[cfg(windows)] extern crate winapi;


mod cal;
pub use crate::cal::{DatePiece, TimePiece};
pub use crate::cal::datetime::{LocalDate, LocalTime, LocalDateTime, Month, Weekday, Year, YearMonth};
pub use crate::cal::fmt::custom as fmt;
pub use crate::cal::fmt::ISO;  // TODO: replace this with just a 'fmt' import
pub use crate::cal::offset::{Offset, OffsetDateTime};
pub use crate::cal::zone::{TimeZone, ZonedDateTime};
pub use crate::cal::zone as zone;

pub use crate::cal::convenience;

mod duration;
pub use crate::duration::Duration;

mod instant;
pub use crate::instant::Instant;

mod system;
pub use crate::system::sys_timezone;

mod util;
