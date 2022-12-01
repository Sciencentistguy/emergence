# Emergence

Emergence is a library to fetch and cache Advent of Code inputs.

The AoC struct is the main entry point for this library.

See `AoC::new` and `AoC::read_or_fetch` for usage

# Example

```rs
fn main() -> Result<(), Box<dyn Error>> {
    let aoc = AoC::new(2020)?; // year 2020
    let input = aoc.read_or_fetch(1)?; // day 01
    solve(&input); // Implementation of `solve` left as an exercise to the reader :)
    Ok(())
}
```

---

Available under the terms of version 2.0 of the Mozilla Public Licence
