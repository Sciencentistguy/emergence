use std::{
    io,
    path::{Path, PathBuf},
};

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use thiserror::Error;

use reqwest::{blocking::Client, header::COOKIE};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("The puzzle has not been released yet. Be patient.")]
    TooSoon,
    #[error("That's a silly date.")]
    InvalidDate,
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
    pub fn new(year: usize) -> Result<Self, Error> {
        let Some(mut path) = dirs::home_dir() else {
            panic!("Could not determine the home directory of the current user. Please set $HOME or use `AoC::with_path` instead.")
        };

        path.push(".aoc");

        Self::with_path(year, path)
    }

    /// Read the input for the specified day from the cache, or if it is not present, fetch it from
    /// Advent of Code
    pub fn read_or_fetch(&self, day: usize) -> Result<String, Error> {
        if let Some(text) = self.read(day)? {
            return Ok(text);
        }

        let text = self.fetch(day)?;
        self.write(day, text.as_str())?;
        Ok(text)
    }

    /// Fetch the input for the specified day from Advent of Code
    fn fetch(&self, day: usize) -> Result<String, Error> {
        let starts = DateTime::<FixedOffset>::from_local(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(
                    self.year.try_into().map_err(|_| Error::TooSoon)?,
                    12,
                    day.try_into().map_err(|_| Error::InvalidDate)?,
                )
                .ok_or(Error::InvalidDate)?,
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ),
            FixedOffset::west_opt(5 * 3600).unwrap(),
        );

        if starts > Utc::now() {
            return Err(Error::TooSoon);
        }

        let res = self
            .client
            .get(format!(
                "https://adventofcode.com/{}/day/{}/input",
                self.year, day
            ))
            .header(COOKIE, format!("session={}", self.token))
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
    use super::*;

    #[test]
    fn cache_create() {
        let dir = tempdir::TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();

        aoc.write(1, "hello").unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.path().join("2020/day01.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn cache_hit() {
        let dir = tempdir::TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();
        aoc.write(1, "hello").unwrap();
        assert_eq!(aoc.read(1).unwrap().unwrap(), "hello");
        assert!(aoc.read(2).unwrap().is_none());
    }

    #[test]
    fn impatient() {
        let dir = tempdir::TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(100_000, dir.path()).unwrap();
        assert!(matches!(aoc.read_or_fetch(1), Err(Error::TooSoon)));
    }

    #[test]
    fn invalid_date() {
        let dir = tempdir::TempDir::new("emergence").unwrap();
        let aoc = AoC::with_path(2020, dir.path()).unwrap();
        assert!(matches!(aoc.read_or_fetch(0), Err(Error::InvalidDate)));
        assert!(matches!(aoc.read_or_fetch(1000), Err(Error::InvalidDate)));
        assert!(matches!(
            aoc.read_or_fetch(usize::max_value()),
            Err(Error::InvalidDate)
        ));
    }
}
