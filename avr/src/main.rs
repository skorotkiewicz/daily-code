#![no_std]
#![no_main]

#[cfg(not(target_os = "linux"))]
compile_error!("the no_std CLI requires Linux; the library is portable");

use core::{ffi::CStr, fmt, fmt::Write};
use daily_code::{CODE_COUNT, DEFAULT_SECRET, DailyCode, Duration, EPOCH, parse_date};
use time::OffsetDateTime;

const USAGE: &str = "Usage:
  daily-code generate [--date YYYY-MM-DD] [--days N]
  daily-code reverse CODE

Defaults: today in UTC, one day, DAILY_CODE_SECRET or demo.
Dates: 1000-01-01 through 3464-02-12. N must be positive.";

struct Output(libc::c_int);

impl fmt::Write for Output {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let mut bytes = text.as_bytes();
        while !bytes.is_empty() {
            // SAFETY: bytes is readable for its length; write does not retain the pointer.
            let count = unsafe { libc::write(self.0, bytes.as_ptr().cast(), bytes.len()) };
            if count < 0 {
                // SAFETY: Linux exposes this thread's errno through a valid pointer.
                if unsafe { *libc::__errno_location() } == libc::EINTR {
                    continue;
                }
                return Err(fmt::Error);
            }
            if count == 0 {
                return Err(fmt::Error);
            }
            bytes = &bytes[count as usize..];
        }
        Ok(())
    }
}

fn today() -> Result<time::Date, &'static str> {
    // SAFETY: a null pointer tells time not to write to an output parameter.
    let seconds = unsafe { libc::time(core::ptr::null_mut()) };
    if seconds == -1 {
        return Err("could not read the UTC clock");
    }
    OffsetDateTime::from_unix_timestamp(seconds as i64)
        .map(|time| time.date())
        .map_err(|_| "UTC clock is outside the supported date range")
}

fn secret() -> Result<&'static str, &'static str> {
    // SAFETY: the key is NUL-terminated. This single-threaded program never mutates the environment.
    let pointer = unsafe { libc::getenv(c"DAILY_CODE_SECRET".as_ptr()) };
    if pointer.is_null() {
        return Ok(DEFAULT_SECRET);
    }
    // SAFETY: getenv returns a NUL-terminated string valid until the environment changes.
    unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .map_err(|_| "DAILY_CODE_SECRET must be UTF-8")
}

// SAFETY: the caller must supply the process's valid, NUL-terminated argv entries.
unsafe fn run(arguments: &[*const libc::c_char]) -> Result<(), &'static str> {
    let mut args = arguments.iter().skip(1).map(|&pointer| {
        // SAFETY: guaranteed by run's caller; the arguments remain valid throughout main.
        unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .map_err(|_| "arguments must be UTF-8")
    });
    let command = args.next().transpose()?.ok_or(USAGE)?;
    let mut output = Output(libc::STDOUT_FILENO);
    if matches!(command, "--help" | "-h") {
        return writeln!(output, "{USAGE}").map_err(|_| "could not write output");
    }
    let generator = DailyCode::new(secret()?);
    match command {
        "generate" => {
            let mut date = None;
            let mut days = None;
            while let Some(option) = args.next().transpose()? {
                match option {
                    "--date" if date.is_none() => {
                        date = Some(parse_date(
                            args.next().transpose()?.ok_or("--date needs a value")?,
                        )?);
                    }
                    "--days" if days.is_none() => {
                        let value = args.next().transpose()?.ok_or("--days needs a value")?;
                        days = Some(
                            value
                                .parse::<u32>()
                                .ok()
                                .filter(|&n| n > 0)
                                .ok_or("--days must be a positive integer")?,
                        );
                    }
                    "--help" | "-h" => {
                        return writeln!(output, "{USAGE}").map_err(|_| "could not write output");
                    }
                    _ => return Err("unknown or repeated generate option; use --help"),
                }
            }
            let day = match date {
                Some(day) => day,
                None => today()?,
            };
            let days = days.unwrap_or(1);
            generator.code_for(day)?;
            let remaining = i64::from(CODE_COUNT) - (day - EPOCH).whole_days();
            if i64::from(days) > remaining {
                return Err("requested days extend beyond 3464-02-12");
            }
            for offset in 0..days {
                let day = day + Duration::days(i64::from(offset));
                let code = generator.code_for(day)?;
                let (first, second) = generator.words_for(day)?;
                writeln!(output, "{code:06}  ({day}, secret: '{first}-{second}')")
                    .map_err(|_| "could not write output")?;
            }
            Ok(())
        }
        "reverse" => {
            let code = args.next().transpose()?.ok_or("reverse needs a code")?;
            if args.next().transpose()?.is_some() {
                return Err("reverse takes exactly one code");
            }
            writeln!(output, "{}", generator.day_from_code(code)?)
                .map_err(|_| "could not write output")
        }
        _ => Err(USAGE),
    }
}

/// # Safety
/// Called only by the C runtime with valid argc and argv.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: libc::c_int, argv: *const *const libc::c_char) -> libc::c_int {
    // SAFETY: the C runtime provides argc valid argument pointers for the duration of main.
    let result = unsafe { run(core::slice::from_raw_parts(argv, argc as usize)) };
    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(Output(libc::STDERR_FILENO), "error: {error}");
            1
        }
    }
}

// Prebuilt core references this symbol even with panic = "abort".
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() -> ! {
    // SAFETY: this binary cannot unwind; abort terminates the process.
    unsafe { libc::abort() }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    let _ = writeln!(
        Output(libc::STDERR_FILENO),
        "fatal: internal daily-code error"
    );
    // SAFETY: abort terminates the process without unwinding across the C runtime.
    unsafe { libc::abort() }
}
