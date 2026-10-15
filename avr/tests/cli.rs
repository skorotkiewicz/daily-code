use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_daily-code"))
        .env("DAILY_CODE_SECRET", "demo")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn original_cli_output() {
    let output = run(&["generate", "--date", "1991-01-1", "--days", "3"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "668558  (1991-01-01, secret: 'vanquish-afraid')\n\
474543  (1991-01-02, secret: 'angriness-alongside')\n\
904406  (1991-01-03, secret: 'stumble-accent')\n"
    );
    for code in ["668558", "668-558", "668 558"] {
        let output = run(&["reverse", code]);
        assert!(output.status.success());
        assert_eq!(output.stdout, b"1991-01-01\n");
    }
}

#[test]
fn secret_and_today() {
    let output = Command::new(env!("CARGO_BIN_EXE_daily-code"))
        .env("DAILY_CODE_SECRET", "another-secret-42")
        .args(["generate", "--date", "1000-01-01"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        b"852991  (1000-01-01, secret: 'footsie-amends')\n"
    );

    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let output = run(&["generate"]);
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let code = text.split_whitespace().next().unwrap();
    let date = daily_code::DailyCode::new("demo")
        .day_from_code(code)
        .unwrap();
    assert!([before, after].into_iter().any(|instant| {
        time::OffsetDateTime::from_unix_timestamp(instant.as_secs() as i64)
            .unwrap()
            .date()
            == date
    }));
}

#[test]
fn invalid_requests_produce_no_partial_output() {
    for args in [
        vec![],
        vec!["generate", "--date", "0999-12-31"],
        vec!["generate", "--date", "3464-02-12", "--days", "2"],
        vec!["generate", "--days", "0"],
        vec!["generate", "--days", "-1"],
        vec!["generate", "--date"],
        vec!["generate", "--date", "1991-02-29"],
        vec!["generate", "--unknown"],
        vec!["reverse", "0100000"],
        vec!["reverse", "1000000"],
        vec!["reverse"],
        vec!["reverse", "668558", "extra"],
    ] {
        let output = run(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
    }
    let output = run(&["--help"]);
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().contains("Usage:"));
}
