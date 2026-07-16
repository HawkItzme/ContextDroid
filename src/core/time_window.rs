use anyhow::{bail, Context, Result};
use std::str::FromStr;

/// A checked, strictly-positive CLI duration represented in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositiveDuration(i64);

impl PositiveDuration {
    pub const DAY: Self = Self(24 * 60 * 60 * 1_000);
    pub const WEEK: Self = Self(7 * Self::DAY.0);
    pub const MONTH: Self = Self(30 * Self::DAY.0);

    pub fn millis(self) -> i64 {
        self.0
    }
}

impl FromStr for PositiveDuration {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        if value.len() < 2 || value.trim() != value {
            bail!("duration must be a positive integer followed by m, h, d, or w");
        }
        let (number, suffix) = value.split_at(value.len() - 1);
        if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
            bail!("duration must be a positive integer followed by m, h, d, or w");
        }
        let amount = number.parse::<i64>().context("duration is too large")?;
        if amount == 0 {
            bail!("duration must be positive");
        }
        let unit_ms = match suffix {
            "m" => 60_000_i64,
            "h" => 60 * 60_000_i64,
            "d" => 24 * 60 * 60_000_i64,
            "w" => 7 * 24 * 60 * 60_000_i64,
            _ => bail!("duration unit must be one of m, h, d, or w"),
        };
        amount
            .checked_mul(unit_ms)
            .map(Self)
            .context("duration is too large")
    }
}

/// A validated post-filter execution limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastCount(u16);

impl LastCount {
    pub fn new(value: usize) -> Result<Self> {
        if !(1..=1_000).contains(&value) {
            bail!("--last must be between 1 and 1000");
        }
        Ok(Self(value as u16))
    }

    pub fn get(self) -> usize {
        usize::from(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_duration_accepts_typed_units_and_equivalent_windows() {
        assert_eq!(
            "60m".parse::<PositiveDuration>().unwrap(),
            "1h".parse().unwrap()
        );
        assert_eq!(
            "24h".parse::<PositiveDuration>().unwrap(),
            "1d".parse().unwrap()
        );
        assert_eq!(
            "7d".parse::<PositiveDuration>().unwrap(),
            "1w".parse().unwrap()
        );
        assert_eq!(
            "2w".parse::<PositiveDuration>().unwrap().millis(),
            1_209_600_000
        );
    }

    #[test]
    fn positive_duration_rejects_ambiguous_invalid_and_overflow_values() {
        for value in [
            "10",
            "m",
            "1x",
            "1.5h",
            " 1h",
            "1h ",
            "0d",
            "-1d",
            "999999999999999999999999999w",
        ] {
            assert!(
                value.parse::<PositiveDuration>().is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn last_count_is_bounded() {
        assert!(LastCount::new(0).is_err());
        assert_eq!(LastCount::new(1).unwrap().get(), 1);
        assert_eq!(LastCount::new(1_000).unwrap().get(), 1_000);
        assert!(LastCount::new(1_001).is_err());
    }
}
