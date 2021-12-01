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

const CACHE_PATH: &str = "~/.aoc";
static TOKEN: Lazy<String> = Lazy::new(|| {
    std::env::var("TOKEN")
        .or_else(|_| std::fs::read_to_string("tokenfile"))
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
    if !Path::new(CACHE_PATH).exists() {
        std::fs::create_dir(CACHE_PATH)?;
    }
    let year_path = format!("{}/{}", CACHE_PATH, year);
    if !Path::new(year_path.as_str()).exists() {
        std::fs::create_dir(year_path)?;
    }
    Ok(())
}

fn cache_read(year: usize, day: usize) -> Option<String> {
    let path = format!("{}/{}/day{:02}.txt", CACHE_PATH, year, day);
    std::fs::read_to_string(path.as_str()).ok()
}

fn cache_write(year: usize, day: usize, text: &str) -> std::io::Result<()> {
    let path = format!("{}/{}/day{:02}.txt", CACHE_PATH, year, day);
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
