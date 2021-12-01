use std::path::Path;

use thiserror::Error;

use once_cell::sync::Lazy;
use reqwest::{blocking::Client, header::COOKIE, Method};

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

static TOKEN: Lazy<String> = Lazy::new(|| {
    std::env::var("TOKEN")
        .or_else(|_| std::fs::read_to_string("tokenfile").map(|x| x.trim().to_owned()))
        .expect("set `TOKEN` or create `tokenfile`")
});

fn fetch_raw(year: usize, day: usize) -> reqwest::Result<String> {
    let client = Client::new();
    let res = client
        .request(
            Method::GET,
            format!("https://adventofcode.com/{}/day/{}/input", year, day),
        )
        .header(COOKIE, format!("session={}", TOKEN.as_str()))
        .send()?;
    res.text()
}

fn cache_init(year: usize) -> std::io::Result<()> {
    let home = std::env::var("HOME").expect("user has home directory");
    let path = format!("{}/.aoc", home);
    if !Path::new(&path).exists() {
        std::fs::create_dir(&path)?;
    }
    let year_path = format!("{}/{}", path, year);
    if !Path::new(year_path.as_str()).exists() {
        std::fs::create_dir(year_path)?;
    }
    Ok(())
}

fn cache_read(year: usize, day: usize) -> Option<String> {
    let home = std::env::var("HOME").expect("user has home directory");
    let path = format!("{}/.aoc/{}/day{:02}.txt", home, year, day);
    std::fs::read_to_string(path.as_str()).ok()
}

fn cache_write(year: usize, day: usize, text: &str) -> std::io::Result<()> {
    let home = std::env::var("HOME").expect("user has home directory");
    let path = format!("{}/.aoc/{}/day{:02}.txt", home, year, day);
    std::fs::write(path, text)
}

pub fn fetch(year: usize, day: usize) -> Result<String, Error> {
    cache_init(year)?;

    if let Some(text) = cache_read(year, day) {
        return Ok(text);
    }

    let text = fetch_raw(year, day)?;
    cache_write(year, day, text.as_str())?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloads_2020_1() {
        let txt = fetch(2020, 1).unwrap();
        let home = std::env::var("HOME").expect("user has home directory");
        let cached = std::fs::read_to_string(format!("{}/.aoc/2020/day01.txt", home)).unwrap();
        assert_eq!(txt.trim(), cached.trim());
    }
}
