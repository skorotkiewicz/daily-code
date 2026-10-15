#![no_std]

//! Reversible daily codes, compatible with daily_code.py on supported dates.
//! No allocator, filesystem, environment, or clock is needed by the library.

use sha2::{Digest, Sha256};
pub use time::{Date, Duration, Month};

pub const CODE_LOW: u32 = 100_000;
pub const CODE_COUNT: u32 = 900_000;
pub const DEFAULT_SECRET: &str = "demo";
pub const EPOCH: Date = match Date::from_calendar_date(1000, Month::January, 1) {
    Ok(date) => date,
    Err(_) => panic!("invalid epoch"),
};

const WORDS: &str = include_str!("../../eff_long_words.txt");
const DATE_RANGE_ERROR: &str = "date must be between 1000-01-01 and 3464-02-12";

/// Borrows the secret for this generator's lifetime. It never copies or prints it.
pub struct DailyCode<'a> {
    secret: &'a str,
}

impl<'a> DailyCode<'a> {
    pub const fn new(secret: &'a str) -> Self {
        Self { secret }
    }

    fn round_fn(&self, round: u8, half: u32) -> u32 {
        let mut hash = Sha256::new();
        hash.update(self.secret.as_bytes());
        hash.update([b'|', b'0' + round, b'|']);
        let mut digits = [0; 4];
        let mut start = digits.len();
        let mut value = half;
        loop {
            start -= 1;
            digits[start] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        hash.update(&digits[start..]);
        let digest = hash.finalize();
        u32::from(u16::from_be_bytes([digest[0], digest[1]])) & 1023
    }

    fn feistel(&self, value: u32, inverse: bool) -> u32 {
        let (mut left, mut right) = (value >> 10, value & 1023);
        for round in 0..4 {
            (left, right) = if inverse {
                (right ^ self.round_fn(3 - round, left), left)
            } else {
                (right, left ^ self.round_fn(round, right))
            };
        }
        (left << 10) | right
    }

    fn walk(&self, mut value: u32, inverse: bool) -> u32 {
        loop {
            value = self.feistel(value, inverse);
            if (CODE_LOW..CODE_LOW + CODE_COUNT).contains(&value) {
                return value;
            }
        }
    }

    pub fn code_for(&self, day: Date) -> Result<u32, &'static str> {
        Ok(self.walk(day_number(day)? + CODE_LOW, false))
    }

    pub fn day_from_code(&self, code: &str) -> Result<Date, &'static str> {
        let mut value = 0;
        let mut count = 0;
        for byte in code.bytes() {
            match byte {
                b' ' | b'-' => continue,
                b'0'..=b'9' if count < 6 => {
                    value = value * 10 + u32::from(byte - b'0');
                    count += 1;
                }
                _ => return Err("code must be 6 digits, not starting with 0"),
            }
        }
        if count != 6 || !(CODE_LOW..CODE_LOW + CODE_COUNT).contains(&value) {
            return Err("code must be 6 digits, not starting with 0");
        }
        let day = self.walk(value, true) - CODE_LOW;
        Ok(EPOCH + Duration::days(i64::from(day)))
    }

    pub fn words_for(&self, day: Date) -> Result<(&'static str, &'static str), &'static str> {
        let index = self.walk(day_number(day)?, false) as usize;
        let count = WORDS.lines().count();
        // The word list and its order are fixed at compile time for Python compatibility.
        Ok((
            WORDS.lines().nth(index % count).expect("embedded word"),
            WORDS
                .lines()
                .nth((index / count) % count)
                .expect("embedded word"),
        ))
    }
}

fn day_number(day: Date) -> Result<u32, &'static str> {
    let number = (day - EPOCH).whole_days();
    if !(0..i64::from(CODE_COUNT)).contains(&number) {
        return Err(DATE_RANGE_ERROR);
    }
    Ok(number as u32)
}

/// Accepts YYYY-M-D as well as zero-padded YYYY-MM-DD, like the Python CLI.
pub fn parse_date(text: &str) -> Result<Date, &'static str> {
    let error = "invalid date (use YYYY-MM-DD)";
    let mut parts = text.split('-');
    let year = parts.next().ok_or(error)?;
    let month = parts.next().ok_or(error)?;
    let day = parts.next().ok_or(error)?;
    if parts.next().is_some()
        || year.len() != 4
        || year == "0000"
        || month.is_empty()
        || month.len() > 2
        || day.is_empty()
        || day.len() > 2
        || ![year, month, day]
            .iter()
            .all(|part| part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(error);
    }
    Date::from_calendar_date(
        year.parse().map_err(|_| error)?,
        Month::try_from(month.parse::<u8>().map_err(|_| error)?).map_err(|_| error)?,
        day.parse().map_err(|_| error)?,
    )
    .map_err(|_| error)
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_vectors() {
        let generator = DailyCode::new(DEFAULT_SECRET);
        for (date, code, words) in [
            ("1991-01-1", 668558, ("vanquish", "afraid")),
            ("1991-01-2", 474543, ("angriness", "alongside")),
            ("1991-01-3", 904406, ("stumble", "accent")),
            ("1000-01-01", 867095, ("railcar", "alienable")),
            ("3464-02-12", 750863, ("reenact", "ambush")),
        ] {
            let day = parse_date(date).unwrap();
            assert_eq!(generator.code_for(day), Ok(code));
            assert_eq!(generator.code_for(day), generator.code_for(day));
            assert_eq!(generator.words_for(day), Ok(words));
            assert_eq!(generator.day_from_code(&std::format!("{code}")), Ok(day));
        }
        for (secret, expected) in [
            ("another-secret-42", 852991),
            ("sëcret🔑", 432909),
            ("", 387856),
        ] {
            let generator = DailyCode::new(secret);
            assert_eq!(generator.code_for(EPOCH), Ok(expected));
            assert_eq!(
                generator.day_from_code(&std::format!("{expected}")),
                Ok(EPOCH)
            );
        }
    }

    #[test]
    fn validation() {
        let generator = DailyCode::new(DEFAULT_SECRET);
        for text in [
            "",
            "1991-02-29",
            "2000-13-01",
            "0000-01-01",
            "1991-001-1",
            "+991-1-1",
            "1991-1-1-extra",
        ] {
            assert!(parse_date(text).is_err(), "{text}");
        }
        assert!(parse_date("2000-2-29").is_ok());
        assert!(parse_date("1900-2-29").is_err());
        for day in [
            EPOCH - Duration::days(1),
            EPOCH + Duration::days(i64::from(CODE_COUNT)),
        ] {
            assert!(generator.code_for(day).is_err());
            assert!(generator.words_for(day).is_err());
        }
        for code in [
            "",
            "99999",
            "1000000",
            "0668558",
            "000000",
            "abc123",
            "１２３４５６",
        ] {
            assert!(generator.day_from_code(code).is_err(), "{code}");
        }
        assert_eq!(generator.day_from_code("668-558"), parse_date("1991-01-01"));
        assert_eq!(generator.day_from_code("668 558"), parse_date("1991-01-01"));
    }

    #[test]
    #[ignore = "exhaustive; run with cargo test --release -- --include-ignored"]
    fn all_days_are_unique_and_reversible() {
        let generator = DailyCode::new(DEFAULT_SECRET);
        let mut seen = std::vec![false; CODE_COUNT as usize];
        for number in 0..CODE_COUNT {
            let day = EPOCH + Duration::days(i64::from(number));
            let code = generator.code_for(day).unwrap();
            assert!((CODE_LOW..CODE_LOW + CODE_COUNT).contains(&code));
            let index = (code - CODE_LOW) as usize;
            assert!(!seen[index], "duplicate code {code}");
            seen[index] = true;
            assert_eq!(generator.day_from_code(&std::format!("{code}")), Ok(day));
        }
        assert!(seen.into_iter().all(|used| used));
    }
}
