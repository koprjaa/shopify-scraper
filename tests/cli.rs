// Project: shopify-scraper
// File: tests/cli.rs
// Description: CLI exit codes, defaults, files and logging.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
mod common;
use clap::Parser;
use shopify_scraper::Settings;
use std::process::Command;
fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_shopify-scraper"))
}
#[test]
fn help_and_invalid_arguments() {
    assert!(command().arg("--help").output().unwrap().status.success());
    for args in [
        vec![],
        vec!["http://localhost", "-v", "4"],
        vec!["http://localhost", "-c"],
        vec!["http://localhost", "-r", "x"],
    ] {
        assert_eq!(
            command().args(args).output().unwrap().status.code(),
            Some(2)
        );
    }
}
#[test]
fn defaults() {
    let s = Settings::parse_from(["scraper", "https://example.com"]);
    assert_eq!(s.max_retries, 3);
    assert_eq!(s.retry_delay, 5);
    assert_eq!(s.verbosity, 2);
    assert_eq!(s.csv_filename, "shopify_products.csv");
    assert_eq!(s.log_filename, "shopify_scraper.log");
    assert_eq!(s.output_folder.to_str(), Some("shopify_exports"));
}
#[test]
fn empty_catalogue_success_and_custom_filenames() {
    let server = common::Server::new(|_, _| (200, b"{}".to_vec()));
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let out = command()
        .args([
            &server.url,
            "-o",
            temp.path().to_str().unwrap(),
            "-f",
            "out.csv",
            "-l",
            "run.log",
            "-d",
            "0",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let folder = std::fs::read_dir(temp.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(folder.join("images").is_dir());
    assert!(folder.join("127_out.csv").is_file());
    let log = std::fs::read_to_string(folder.join("run.log")).unwrap();
    assert!(log.contains("Product extraction completed successfully"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("Total unique products found: 0"));
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}
#[test]
fn failure_exit_one_and_no_success_message() {
    let server = common::Server::new(|_, _| (429, vec![]));
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let out = command()
        .args([
            &server.url,
            "-o",
            temp.path().to_str().unwrap(),
            "-r",
            "3",
            "-d",
            "0",
            "-v",
            "1",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(server.requests.lock().unwrap().len(), 3);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Failed to retrieve data"));
    assert!(!stderr.contains("successfully"));
    assert!(!stderr.contains("WARNING"));
}

#[test]
fn repeated_options_use_the_last_value_like_argparse() {
    let s = Settings::try_parse_from([
        "scraper",
        "http://localhost",
        "-r",
        "1",
        "-r",
        "2",
        "-c",
        "a",
        "-c",
        "b",
    ])
    .unwrap();
    assert_eq!(s.max_retries, 2);
    assert_eq!(s.collections, ["b"]);
}
