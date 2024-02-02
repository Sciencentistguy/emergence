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

use tap::TapOptional;
use thiserror::Error;

#[cfg(not(miri))]
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
#[cfg(not(miri))]
use reqwest::{
    blocking::Client,
    header::{COOKIE, USER_AGENT},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Refusing to fetch input for day {0}, as it has not yet been released")]
    NotYetReleased(usize),
    #[error("Advent of Code problems are 1-indexed, day 0 does not exist")]
    DayZero,
    #[error("Advent of Code stops after the 25th")]
    OutOfBounds,
}

/// The AoC struct is the main entry point for this library.
///
/// See [`AoC::new`] and [`AoC::read_or_fetch`] for usage
pub struct AoC {
    path: PathBuf,
    token: String,
    year: usize,

    #[cfg(not(miri))]
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
        assert!(year < 3000, "Year must be less than 3000");
        std::fs::create_dir_all(path.as_ref().join(year.to_string()))?;
        Ok(Self {
            path: path.as_ref().to_owned(),
            year,
            token,

            #[cfg(not(miri))]
            client: Client::new(),
        })
    }

    /// Find a `./tokenfile` in the current directory, or search upwards recursively
    fn find_tokenfile() -> Result<Option<PathBuf>, Error> {
        let mut path = std::env::current_dir()?;
        while {
            let tokenpath = path.join("tokenfile");
            if tokenpath.is_file() {
                return Ok(Some(tokenpath));
            }
            path.pop()
        } {}
        Ok(None)
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
        let tokenpath = Self::find_tokenfile()?;

        let Some(token) = std::env::var("TOKEN").ok().or_else(|| {
            tokenpath
                .and_then(|tokenpath| std::fs::read_to_string(tokenpath).ok())
                .tap_some_mut(|s| s.truncate(s.trim_end().len()))
        }) else {
            panic!("Could not read token from $TOKEN or find a ./tokenfile in this directory or any parent. Please set the token in one of these locations or use `AoC::with_path_and_token`");
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
    #[cfg(not(miri))]
    pub fn new(year: usize) -> Result<Self, Error> {
        let Some(mut path) = dirs::home_dir() else {
            panic!("Could not determine the home directory of the current user. Please set $HOME or use `AoC::with_path` instead.")
        };

        path.push(".aoc");

        Self::with_path(year, path)
    }

    #[cfg(miri)]
    pub fn new(year: usize) -> Result<Self, Error> {
        panic!(
            "When running under miri, you must use `AoC::with_path` or `AoC::with_path_and_token`, as it is impossible to discover the user's home directory."
        );
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
            return Err(Error::DayZero);
        }
        if day > 25 {
            return Err(Error::OutOfBounds);
        }

        if let Some(text) = self.read(day)? {
            return Ok(text);
        }

        #[cfg(miri)]
        {
            eprintln!("Cannot fetch input under miri, and it is not present in the cache. Exiting");
            std::process::exit(1);
        }

        #[cfg(not(miri))]
        {
            let text = self.fetch(day)?;
            self.write(day, text.as_str())?;
            Ok(text)
        }
    }

    /// Fetch the input for the specified day from Advent of Code
    #[cfg(not(miri))]
    fn fetch(&self, day: usize) -> Result<String, Error> {
        let starts = DateTime::<FixedOffset>::from_naive_utc_and_offset(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(self.year as _, 12, day as _).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ),
            FixedOffset::west_opt(5 * 60 * 60).unwrap(),
        );

        if starts > Utc::now() {
            return Err(Error::NotYetReleased(day));
        }

        let res = self
            .client
            .get(format!(
                "https://adventofcode.com/{}/day/{}/input",
                self.year, day
            ))
            .header(COOKIE, format!("session={}", self.token))
            .header(
                USER_AGENT,
                "github.com/Sciencentistguy/emergence by jamie@quigley.xyz",
            )
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
    fn cache_miss() {
        let dir = TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();
        assert!(aoc.read(1).unwrap().is_none());
        assert_ne!(aoc.fetch(1).unwrap().len(), 0);
    }

    #[test]
    #[should_panic]
    fn future() {
        let dir = TempDir::new("emergence").unwrap();
        let _aoc = AoC::with_path(100_000, dir.path()).unwrap();
    }

    #[test]
    fn day00() {
        let dir = TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();
        assert!(matches!(aoc.read_or_fetch(0), Err(Error::DayZero)));
    }
    #[test]
    fn day31() {
        let dir = TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();
        assert!(matches!(aoc.read_or_fetch(31), Err(Error::OutOfBounds)));
    }

    #[test]
    fn finds_tokenfile() {
        let cwd = std::env::current_dir().unwrap();

        let dir = TempDir::new("emergence").unwrap();
        let mut dir = dir.path().to_owned();
        std::env::set_current_dir(&dir).unwrap();

        std::fs::write(dir.join("tokenfile"), "TESTTOKEN").unwrap();
        assert!(AoC::find_tokenfile().unwrap().is_some());

        dir.push("a");
        dir.push("b");
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_current_dir(&dir).unwrap();

        assert!(AoC::find_tokenfile().unwrap().is_some());

        std::env::set_current_dir(cwd).unwrap();
    }
}
