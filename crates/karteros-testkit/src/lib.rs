//! Explicit process fault-injection helpers for Karteros tests and examples.

#![forbid(unsafe_code)]

use std::{env, error::Error, fmt, str::FromStr};

pub const CRASH_POINT_ENV: &str = "KARTEROS_CRASH_POINT";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrashPoint {
    AfterSenderCommit,
    AfterExternalEffect,
}

impl CrashPoint {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AfterSenderCommit => "after-sender-commit",
            Self::AfterExternalEffect => "after-external-effect",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CrashPointError {
    Unknown(String),
}

impl fmt::Display for CrashPointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(value) => write!(formatter, "unknown Karteros crash point: {value}"),
        }
    }
}

impl Error for CrashPointError {}

impl FromStr for CrashPoint {
    type Err = CrashPointError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "after-sender-commit" => Ok(Self::AfterSenderCommit),
            "after-external-effect" => Ok(Self::AfterExternalEffect),
            _ => Err(CrashPointError::Unknown(value.to_owned())),
        }
    }
}

pub fn parse_selection(value: Option<&str>) -> Result<Option<CrashPoint>, CrashPointError> {
    value.map(CrashPoint::from_str).transpose()
}

pub fn abort_if_configured(reached: CrashPoint) -> Result<(), CrashPointError> {
    let configured = env::var(CRASH_POINT_ENV).ok();
    if parse_selection(configured.as_deref())? == Some(reached) {
        eprintln!("Karteros fault injection reached {}", reached.as_str());
        std::process::abort();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{CrashPoint, CrashPointError, parse_selection};

    #[test]
    fn crash_point_names_round_trip() {
        for point in [
            CrashPoint::AfterSenderCommit,
            CrashPoint::AfterExternalEffect,
        ] {
            assert_eq!(CrashPoint::from_str(point.as_str()).unwrap(), point);
        }
    }

    #[test]
    fn optional_selection_distinguishes_absent_and_unknown_values() {
        assert_eq!(parse_selection(None).unwrap(), None);
        assert_eq!(
            parse_selection(Some("after-sender-commit")).unwrap(),
            Some(CrashPoint::AfterSenderCommit)
        );
        assert_eq!(
            parse_selection(Some("surprise")),
            Err(CrashPointError::Unknown("surprise".to_owned()))
        );
    }
}
