# daily-code in Rust

A `#![no_std]` port of `../daily_code.py`. It generates one reversible six-digit
code per day and the original EFF word pair, keyed by `DAILY_CODE_SECRET`.
The supported dates are 1000-01-01 through 3464-02-12.

## Current status

The release build and all six tests pass, including exhaustive uniqueness and
reversal checks across all 900,000 supported days.

The word list is bundled as `src/eff_long_words.txt` and embedded with
`include_str!("./eff_long_words.txt")`. The path is relative to `src/lib.rs`,
not the directory where the executable runs.

Despite the directory name, the current CLI targets Linux, not AVR hardware.

## Requirements

- Rust with edition 2024 support and Cargo.
- A Linux host with a C linker and libc for the CLI.
- The bundled `src/eff_long_words.txt` available at compile time.

Both the library and CLI use `no_std` and require no Rust heap allocation.
The CLI uses libc for startup, arguments, environment variables, UTC time, and
output. The library accepts a secret and date from its caller and needs no OS.
Tests use Rust's standard library.

## Build and run

Run these commands from this directory, `avr/`:

```sh
cargo build --release
./target/release/daily-code generate
./target/release/daily-code generate --date 1991-01-1
./target/release/daily-code generate --date 1991-01-1 --days 3
./target/release/daily-code reverse 668558
./target/release/daily-code reverse 668-558
```

For `generate --date 1991-01-1`, the expected output is:

```text
668558  (1991-01-01, secret: 'vanquish-afraid')
```

Without `--date`, generation uses today's UTC date. `--days` must be positive.
An unsupported date range is rejected before any output is printed.
The embedded word list requires no runtime download or cache.

## Secret

```sh
DAILY_CODE_SECRET=my-secret ./target/release/daily-code generate
```

The default is `demo`. Keep the secret private and stable, since changing it
changes the code mapping. Reverse lookup requires the same secret used to
generate the code. The printed word pair is not the actual secret.

Six-digit codes and this custom permutation are not a substitute for
authentication.

## Library

```rust
use daily_code::{DailyCode, parse_date};

let generator = DailyCode::new("demo");
let day = parse_date("1991-01-1").unwrap();
assert_eq!(generator.code_for(day), Ok(668558));
assert_eq!(generator.day_from_code("668-558"), Ok(day));
assert_eq!(generator.words_for(day), Ok(("vanquish", "afraid")));
```

The generator borrows its secret for its lifetime. Dates before the epoch,
invalid calendar dates, and codes with extra leading zeros are rejected.

## Checks

```sh
cargo test
cargo test --release -- --include-ignored
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo check --lib --target wasm32-unknown-unknown
```

The exhaustive release test checks uniqueness and reversal across all 900,000
supported days. The last command checks only the library and requires the
`wasm32-unknown-unknown` target to be installed. It is not an AVR hardware check.
Formatting, Clippy, and the cross-target check have not yet been verified.
The original Python implementation and tests remain in the parent directory.
