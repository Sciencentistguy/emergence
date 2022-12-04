//! Emergence is a library to fetch and cache Advent of Code inputs.
//! 
//! The [`AoC`] struct is the main entry point for this library.
//! 
//! See [`AoC::new`] and [`AoC::read_or_fetch`] for usage
//!
//! # Example 
//!
//! ```
//! # use emergence::AoC;
//! # use std::error::Error;
//! # fn solve(input: &str) {}
//! fn main() -> Result<(), Box<dyn Error>> {
//!     let aoc = AoC::new(2020)?; // year 2020
//!     let input = aoc.read_or_fetch(1)?; // day 01
//!     solve(&input); // Implementation of `solve` left as an exercise to the reader :)
//!     Ok(())
//! }
//! ```
use std::{
    io,
    path::{Path, PathBuf},
};

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use thiserror::Error;

use reqwest::{blocking::Client, header::{COOKIE, USER_AGENT}};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// The AoC struct is the main entry point for this library.
///
/// See [`AoC::new`] and [`AoC::read_or_fetch`] for usage
pub struct AoC {
    path: PathBuf,
    token: String,
    year: usize,
    client: Client,
}

impl AoC {
    /// Constructs a new AoC instance at the specified path with the given token
    ///
    /// # Panics
    ///
    /// Will panic if:
    /// - `year` is more than 3000 (if this is a problem for you, please open an issue. I'm
    /// impressed Advent of Code is still going tbh)
    pub fn with_path_and_token(
        year: usize,
        path: impl AsRef<Path>,
        token: String,
    ) -> Result<Self, Error> {
        std::fs::create_dir_all(path.as_ref().join(year.to_string()))?;
        Ok(Self {
            path: path.as_ref().to_owned(),
            year,
            token,
            client: Client::new(),
        })
    }

    /// Constructs a new AoC instance at the specified path, reading the token from `$TOKEN`
    /// or `./tokenfile`
    ///
    /// # Panics
    ///
    /// Will panic if:
    /// - `year` is more than 3000 (if this is a problem for you, please open an issue. I'm
    /// impressed Advent of Code is still going tbh)
    pub fn with_path(year: usize, path: impl AsRef<Path>) -> Result<Self, Error> {
        let Ok(token) = std::env::var("TOKEN")
            .or_else(|_| std::fs::read_to_string("tokenfile").map(|x| x.trim().to_owned()))
            else {
                panic!("Could not read token from $TOKEN or ./tokenfile. Please set the token in one of these locations or use `AoC::with_path_and_token`");
            };

        Self::with_path_and_token(year, path, token)
    }

    /// Construct a new AoC instance in the current user's home directory (see [`dirs::home_dir`]),
    /// reading the token from `$TOKEN` or `./tokenfile`
    ///
    /// [`dirs::home_dir`]: https://docs.rs/dirs/4.0.0/dirs/fn.home_dir.html
    ///
    /// # Panics
    ///
    /// Will panic if:
    /// - `year` is more than 3000 (if this is a problem for you, please open an issue. I'm
    /// impressed Advent of Code is still going tbh)
    pub fn new(year: usize) -> Result<Self, Error> {
        let Some(mut path) = dirs::home_dir() else {
            panic!("Could not determine the home directory of the current user. Please set $HOME or use `AoC::with_path` instead.")
        };

        path.push(".aoc");

        Self::with_path(year, path)
    }

    /// Read the input for the specified day from the cache, or if it is not present, fetch it from
    /// Advent of Code
    ///
    /// # Panics
    ///
    /// Will panic if:
    /// - `day` is 0
    /// - `day` is more than 25
    /// - The puzzle for `day` has not been released yet
    pub fn read_or_fetch(&self, day: usize) -> Result<String, Error> {
        if day == 0 {
            panic!("The first puzzle is day 01. Not fetch day 00.");
        }
        if day > 25 {
            panic!("There are only 25 days in Advent of Code. Not fetching day {day:02}.");
        }

        if let Some(text) = self.read(day)? {
            return Ok(text);
        }

        let text = self.fetch(day)?;
        self.write(day, text.as_str())?;
        Ok(text)
    }

    /// Fetch the input for the specified day from Advent of Code
    fn fetch(&self, day: usize) -> Result<String, Error> {
        let starts = DateTime::<FixedOffset>::from_utc(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(self.year as _, 12, day as _).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ),
            FixedOffset::west_opt(5 * 60 * 60).unwrap(),
        );

        if starts > Utc::now() {
            panic!("Not fetching puzzle for day {day:02}, as it has not been released yet.",);
        }

        let res = self
            .client
            .get(format!(
                "https://adventofcode.com/{}/day/{}/input",
                self.year, day
            ))
            .header(COOKIE, format!("session={}", self.token))
            .header(USER_AGENT, "github.com/Sciencentistguy/emergence by jamie@quigley.xyz")
            .send()?
            .error_for_status()?;
        Ok(res.text()?)
    }

    /// Read the input for the specified day from the cache
    fn read(&self, day: usize) -> io::Result<Option<String>> {
        let path = self.loc(day);
        if !path.exists() {
            return Ok(None);
        }
        std::fs::read_to_string(path).map(Some)
    }

    /// Read the given text for the specified day to the cache
    fn write(&self, day: usize, text: &str) -> io::Result<()> {
        std::fs::write(self.loc(day), text)
    }

    /// The location of the cached input (or where it would be cached) for the specified day
    fn loc(&self, day: usize) -> PathBuf {
        let mut path = self.path.clone();
        path.push(self.year.to_string());
        path.push(format!("day{:02}.txt", day));
        path
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::*;

    #[test]
    fn cache_create() {
        let dir = TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();

        aoc.write(1, "hello").unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.path().join("2020/day01.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn cache_hit() {
        let dir = TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();
        aoc.write(1, "hello").unwrap();
        assert_eq!(aoc.read(1).unwrap().unwrap(), "hello");
        assert!(aoc.read(2).unwrap().is_none());
    }

    #[test]
    #[should_panic]
    fn future() {
        let dir = TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(100_000, dir.path()).unwrap();
        let _ = aoc.read_or_fetch(1);
    }

    #[test]
    #[should_panic]
    fn day00() {
        let dir = TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();
        let _ = aoc.read_or_fetch(0);
    }
    #[test]
    #[should_panic]
    fn day31() {
        let dir = TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();
        let _ = aoc.read_or_fetch(31);
    }
}
