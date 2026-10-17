// Project: shopify-scraper
// File: tests/audit.rs
// Description: Regression cases confirmed by differential runs of the original CLI.
// Author: Jan Alexandr Kopřiva
// License: MIT
mod common;
use clap::Parser;
use serde_json::json;
use shopify_scraper::{
    http_client_with_timeout, products::*, retry_delay, Logger, Scraper, Settings,
};
use std::time::{Duration, Instant};

#[test]
fn iterable_tags_match_python_join() {
    for (tags, expected) in [(json!("až"), "a, ž"), (json!({"z":1,"a":2}), "z, a")] {
        assert_eq!(product_fields(&json!({"tags":tags}), "")["TAGS"], expected);
    }
}

#[test]
fn python_container_and_exponent_strings() {
    for (input, expected) in [
        (json!([true, null, "x"]), "[True, None, 'x']"),
        (json!({"z":true,"a":null}), "{'z': True, 'a': None}"),
        (json!(["a'b", "x\n\\"]), "[\"a'b\", 'x\\n\\\\']"),
        (json!(1e-7), "1e-07"),
        (json!(1e-5), "1e-05"),
        (json!(["\u{200b}", "\u{a0}"]), "['\\u200b', '\\xa0']"),
    ] {
        assert_eq!(remove_html_tags(&input), expected);
        assert_eq!(cell(&input), expected);
    }
}

#[test]
fn malformed_catalogues_are_errors_instead_of_successful_empty_exports() {
    for payload in [
        json!({"products":42}),
        json!({"products":[null]}),
        json!([]),
        json!({"collections":42}),
    ] {
        let body = payload.to_string().into_bytes();
        let server = common::Server::new(move |_, _| (200, body.clone()));
        let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
        let mut settings = Settings::parse_from(["scraper", &server.url, "-d", "0"]);
        if payload.get("collections").is_none() {
            settings.collections = vec!["socks".into()];
        }
        let scraper =
            Scraper::new(settings, Logger::new(&temp.path().join("log"), 1).unwrap()).unwrap();
        assert!(scraper.collect().is_err(), "{payload}");
        assert_eq!(server.requests.lock().unwrap().len(), 1);
    }
}

#[test]
fn stalled_http_responses_have_a_deadline() {
    let server = common::Server::new(|_, _| {
        std::thread::sleep(Duration::from_millis(250));
        (200, b"{}".to_vec())
    });
    let client = http_client_with_timeout(Duration::from_millis(50)).unwrap();
    let start = Instant::now();
    assert!(client.get(&server.url).send().unwrap_err().is_timeout());
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn retry_arithmetic_is_checked() {
    assert!(retry_delay(i64::MAX, 1).is_err());
    assert!(retry_delay(1, 64).is_err());
    assert_eq!(retry_delay(0, 64).unwrap(), Duration::ZERO);
}

#[test]
fn json_bom_and_utf16_are_decoded_before_parsing() {
    let text = "{\"products\":[{\"title\":\"Žluťoučký\",\"variants\":[{}]}]}";
    let mut utf8 = vec![0xef, 0xbb, 0xbf];
    utf8.extend_from_slice(text.as_bytes());
    let mut utf16 = vec![0xff, 0xfe];
    for unit in text.encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    let mut utf32le = vec![0xff, 0xfe, 0, 0];
    let mut utf32be = vec![0, 0, 0xfe, 0xff];
    for c in text.chars() {
        utf32le.extend_from_slice(&(c as u32).to_le_bytes());
        utf32be.extend_from_slice(&(c as u32).to_be_bytes());
    }
    for body in [utf8, utf16, utf32le, utf32be] {
        let server = common::Server::new(move |_, _| (200, body.clone()));
        let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
        let settings = Settings::parse_from(["scraper", &server.url, "-r", "1"]);
        let scraper =
            Scraper::new(settings, Logger::new(&temp.path().join("log"), 1).unwrap()).unwrap();
        assert_eq!(
            scraper.get_page(&server.url, 1, None).unwrap()[0]["title"],
            "Žluťoučký"
        );
    }
}

#[test]
fn netrc_unquoted_escapes_match_python() {
    assert_eq!(
        shopify_scraper::auth::parse(
            "machine localhost login two\\ words password p\\ ass",
            "localhost"
        ),
        Some(("two words".into(), "p ass".into()))
    );
}
