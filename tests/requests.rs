// Project: shopify-scraper
// File: tests/requests.rs
// Description: Request, pagination, retry and extraction regression tests.
// Author: Jan Alexandr Kopřiva
// License: MIT
mod common;
use clap::Parser;
use common::Server;
use serde_json::json;
use shopify_scraper::*;
use std::time::Duration;
fn scraper(server: &Server, temp: &tempfile::TempDir, handles: &[&str]) -> Scraper {
    let mut s = Settings::parse_from(["shopify-scraper", &server.url, "-d", "0"]);
    s.output_folder = temp.path().into();
    s.collections = handles.iter().map(|s| s.to_string()).collect();
    Scraper::new(s, Logger::new(&temp.path().join("test.log"), 3).unwrap()).unwrap()
}
#[test]
fn refused_requests_use_all_attempts_and_name_url() {
    let server = Server::new(|_, _| (429, vec![]));
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let s = scraper(&server, &temp, &[]);
    let url = format!("{}/products.json", server.url);
    let e = s.make_request(&url).unwrap_err().to_string();
    assert!(e.contains(&url));
    assert_eq!(server.requests.lock().unwrap().len(), 3);
    assert!(s.get_page(&server.url, 1, None).is_err());
    assert!(s.collect().is_err());
}
#[test]
fn successful_payload_empty_missing_and_collection_endpoint() {
    let server = Server::new(|_, i| {
        (
            200,
            match i {
                0 => br#"{"products":[{"id":1}]}"#.to_vec(),
                1 => br#"{"products":[]}"#.to_vec(),
                _ => b"{}".to_vec(),
            },
        )
    });
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let s = scraper(&server, &temp, &[]);
    assert_eq!(
        s.make_request(&server.url).unwrap(),
        json!({"products":[{"id":1}]})
    );
    assert!(s.get_page(&server.url, 1, None).unwrap().is_empty());
    assert!(s
        .get_page(&server.url, 1, Some("socks"))
        .unwrap()
        .is_empty());
    assert!(server.requests.lock().unwrap()[2]
        .starts_with("GET /collections/socks/products.json?page=1 "));
    assert!(server.requests.lock().unwrap()[0].contains(USER_AGENT));
}
fn catalogue(path: &str) -> (u16, Vec<u8>) {
    let data = if path.starts_with("/collections.json") {
        if path.ends_with("page=1") {
            json!({"collections":[{"handle":"listed"}]})
        } else {
            json!({"collections":[]})
        }
    } else if path.ends_with("page=1") {
        json!({"products":[{"title":"Sock","handle":"sock","images":[],"variants":[{"title":"Default Title","price":"10.00"},{"title":"Blue","sku":"B"}]}]})
    } else {
        json!({"products":[]})
    };
    (200, data.to_string().into_bytes())
}
#[test]
fn explicit_unlisted_and_multiple_collections_bypass_listing_and_export_all_variants() {
    let server = Server::new(|p, _| catalogue(p));
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let s = scraper(&server, &temp, &["socks", "shoes"]);
    let folder = output_folder(&s.settings).unwrap();
    s.run(&folder).unwrap();
    let seen = server.requests.lock().unwrap();
    assert!(!seen.iter().any(|r| r.contains("/collections.json")));
    for h in ["socks", "shoes"] {
        assert!(seen
            .iter()
            .any(|r| r.contains(&format!("/collections/{h}/products.json"))));
    }
    let mut reader = csv::Reader::from_path(folder.join("127_shopify_products.csv")).unwrap();
    let rows = reader
        .records()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(&rows[1][0], "Sock - Blue");
}
#[test]
fn automatic_listing_is_lazy_and_stops_on_empty_page() {
    let server = Server::new(|p, _| catalogue(p));
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let s = scraper(&server, &temp, &[]);
    assert_eq!(s.collect().unwrap().len(), 2);
    let seen = server.requests.lock().unwrap();
    assert_eq!(seen.len(), 4);
    assert!(seen[0].contains("/collections.json?page=1"));
    assert!(seen[1].contains("/collections/listed/products.json?page=1"));
    assert!(seen[3].contains("/collections.json?page=2"));
}
#[test]
fn store_names() {
    for (url, name) in [
        ("https://www.allbirds.com", "allbirds"),
        ("https://kith.com", "kith"),
        ("https://demo.myshopify.com", "demo"),
        ("https://www.shop.example.co.uk", "shop"),
        ("http://www.example.com/collections/all", "example"),
        ("https://WWW.Example.COM", "example"),
        ("not-a-url", "store"),
    ] {
        assert_eq!(store_name_from_url(url), name);
    }
    assert_ne!(
        store_name_from_url("https://www.allbirds.com"),
        store_name_from_url("https://www.gymshark.com")
    );
}
#[test]
fn exponential_backoff() {
    assert_eq!(
        [retry_delay(5, 0).unwrap(), retry_delay(5, 1).unwrap()],
        [Duration::from_secs(5), Duration::from_secs(10)]
    );
    assert!(retry_delay(-1, 0).is_err());
}
#[test]
fn transient_errors_and_invalid_json_are_retried() {
    let server = Server::new(|_, i| match i {
        0 => (429, vec![]),
        1 => (200, b"bad json".to_vec()),
        _ => (200, b"{}".to_vec()),
    });
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let s = scraper(&server, &temp, &[]);
    assert_eq!(s.make_request(&server.url).unwrap(), json!({}));
    assert_eq!(server.requests.lock().unwrap().len(), 3);
}
#[test]
fn images_and_nonfatal_network_errors() {
    let server = Server::new(|p, _| {
        if p.starts_with("/a.jpg") {
            (200, b"image bytes".to_vec())
        } else {
            (429, vec![])
        }
    });
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let s = scraper(&server, &temp, &[]);
    assert_eq!(
        s.download_image(&format!("{}/a.jpg?x=1", server.url), temp.path())
            .unwrap()
            .as_deref(),
        Some("a.jpg")
    );
    assert_eq!(
        std::fs::read(temp.path().join("a.jpg")).unwrap(),
        b"image bytes"
    );
    assert!(s
        .download_image(&format!("{}/bad", server.url), temp.path())
        .unwrap()
        .is_none());
    assert!(s.download_image("", temp.path()).unwrap().is_none());
}
#[test]
fn nonpositive_attempts_do_not_request() {
    let server = Server::new(|_, _| panic!("must not request"));
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let mut s = scraper(&server, &temp, &[]);
    s.settings.max_retries = 0;
    assert!(s.make_request(&server.url).is_err());
    assert!(server.requests.lock().unwrap().is_empty());
}

#[test]
fn actual_retry_sleeps_only_between_attempts() {
    let server = Server::new(|_, _| (429, vec![]));
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let mut s = scraper(&server, &temp, &[]);
    s.settings.retry_delay = 5;
    let mut slept = Vec::new();
    assert!(s
        .make_request_with_sleep(&server.url, |d| slept.push(d))
        .is_err());
    assert_eq!(slept, [Duration::from_secs(5), Duration::from_secs(10)]);
    let server = Server::new(|_, _| (200, b"{}".to_vec()));
    slept.clear();
    s.make_request_with_sleep(&server.url, |d| slept.push(d))
        .unwrap();
    assert!(slept.is_empty());
}
#[test]
fn colliding_image_names_are_reserved_before_download() {
    let urls = [
        "http://localhost/a/x.jpg",
        "http://localhost/b/x.jpg",
        "http://localhost/c/x_2.jpg",
        "http://localhost/",
    ]
    .map(str::to_owned);
    assert_eq!(
        image_names(&urls),
        ["x.jpg", "x_2.jpg", "x_2_2.jpg", "image"]
    );
}

#[test]
fn netrc_machine_default_and_quoted_tokens() {
    let user = format!("user{}", std::process::id());
    let value = format!("value{}", std::process::id());
    let text = format!("# comment\ndefault login fallback password \"{value}\"\nmachine localhost login \"{user}\" password \"{value}\"\nmacdef ignored\nignored commands\n\n");
    assert_eq!(auth::parse(&text, "localhost"), Some((user, value.clone())));
    assert_eq!(
        auth::parse(&text, "other"),
        Some(("fallback".into(), value))
    );
    assert!(auth::parse("machine", "localhost").is_none());
}

#[test]
fn export_downloads_shared_urls_once_and_preserves_colliding_image_bytes() {
    use std::sync::{Arc, Mutex};
    let base = Arc::new(Mutex::new(String::new()));
    let endpoint = base.clone();
    let server =
        Server::new(move |path, _| {
            if path.contains("products.json") {
                if path.ends_with("page=2") {
                    return (200, br#"{"products":[]}"#.to_vec());
                }
                let host = endpoint.lock().unwrap();
                let products = ["one", "two", "three"].iter().enumerate().map(|(i, handle)| json!({
                "title": handle, "handle": handle,
                "images":[{"src":format!("{host}/{}/x.jpg", if i < 2 {"a"} else {"b"})}],
                "variants":[{"title":"Default Title"},{"title":"Blue"}]
            })).collect::<Vec<_>>();
                return (200, json!({"products":products}).to_string().into_bytes());
            }
            (200, path.as_bytes().to_vec())
        });
    *base.lock().unwrap() = server.url.clone();
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let s = scraper(&server, &temp, &["socks"]);
    let folder = output_folder(&s.settings).unwrap();
    s.run(&folder).unwrap();
    assert_eq!(
        std::fs::read(folder.join("images/x.jpg")).unwrap(),
        b"/a/x.jpg"
    );
    assert_eq!(
        std::fs::read(folder.join("images/x_2.jpg")).unwrap(),
        b"/b/x.jpg"
    );
    let mut csv = csv::Reader::from_path(folder.join("127_shopify_products.csv")).unwrap();
    let rows = csv
        .records()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 6);
    for row in &rows[..4] {
        assert_eq!(&row[19], "x.jpg");
    }
    for row in &rows[4..] {
        assert_eq!(&row[19], "x_2.jpg");
    }
    let requests = server.requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|s| s.starts_with("GET /a/x.jpg "))
            .count(),
        1
    );
    assert_eq!(
        requests
            .iter()
            .filter(|s| s.starts_with("GET /b/x.jpg "))
            .count(),
        1
    );
}

#[test]
fn malformed_optional_fields_are_empty_and_logger_does_not_leak_handlers() {
    use shopify_scraper::products::*;
    let p = json!({"title":"X","tags":7,"images":[{}],"variants":"invalid"});
    let fields = product_fields(&p, "http://localhost");
    assert_eq!(fields["TAGS"], "");
    assert_eq!(fields["IMGURL"], "");
    assert!(variant_rows(&p, "http://localhost").is_empty());
    assert_eq!(
        product_fields(&json!({"tags":[1,true]}), "")["TAGS"],
        "1, True"
    );
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let path = temp.path().join("log");
    for _ in 0..2 {
        let logger = Logger::new(&path, 2).unwrap();
        logger.log("INFO", "Žluťoučký kůň");
        logger.log("DEBUG", "hidden");
    }
    let log = std::fs::read_to_string(path).unwrap();
    assert_eq!(log.lines().count(), 2);
    assert_eq!(log.matches("Žluťoučký kůň").count(), 2);
}

#[test]
fn repeated_exports_never_reuse_output_directories() {
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let mut s = Settings::parse_from(["scraper", "http://localhost"]);
    s.output_folder = temp.path().into();
    let first = output_folder(&s).unwrap();
    std::fs::write(first.join("marker"), b"keep").unwrap();
    let second = output_folder(&s).unwrap();
    assert_ne!(first, second);
    assert!(second.join("images").is_dir());
    assert_eq!(std::fs::read(first.join("marker")).unwrap(), b"keep");
}

#[test]
fn output_names_preserve_urlparse_hostname_spelling() {
    for (url, name) in [
        ("https://www.žluťoučký.cz", "žluťoučký"),
        ("//www.example.com/path", "example"),
        ("https://user@www.example.com:invalid/path", "example"),
        ("https://[::1]:8080", "::1"),
        ("https:example.com", "store"),
        ("https://www.", "store"),
    ] {
        assert_eq!(store_name_from_url(url), name);
    }
}
