// Project: shopify-scraper
// File: tests/products.rs
// Description: Equivalents of every original product test.
// Author: Jan Alexandr Kopřiva
// License: MIT
use serde_json::{json, Value};
use shopify_scraper::products::*;
fn product() -> Value {
    serde_json::from_str(include_str!("fixtures/product.json")).unwrap()
}
const STORE: &str = "https://shop.example.com";
#[test]
fn descriptions() {
    for (v, expected) in [
        (
            json!("<p>Lehké boty <strong>na běh</strong>.</p>"),
            "Lehké boty na běh.",
        ),
        (json!(""), ""),
        (Value::Null, ""),
        (json!("Plain text"), "Plain text"),
        (json!(123), "123"),
        (json!(0), ""),
        (json!("&amp;<br>"), "&amp;"),
    ] {
        assert_eq!(remove_html_tags(&v), expected);
    }
}
#[test]
fn flags() {
    for v in [json!(true), json!(1), json!("text"), json!([1])] {
        assert_eq!(yes_no(&v), "Yes");
    }
    for v in [
        json!(false),
        json!(0),
        json!(""),
        Value::Null,
        json!([]),
        json!({}),
    ] {
        assert_eq!(yes_no(&v), "No");
    }
}
#[test]
fn names() {
    for (v, e) in [
        (json!("42"), "Boty - 42"),
        (json!("Default Title"), "Boty"),
        (json!(""), "Boty"),
        (Value::Null, "Boty"),
        (json!("Black / L"), "Boty - Black / L"),
    ] {
        assert_eq!(variant_name(&json!("Boty"), &v), e);
    }
    assert_eq!(variant_name(&json!("- X"), &json!("Y -")), "X - Y");
}
#[test]
fn shared_fields() {
    let r = product_fields(&product(), STORE);
    for (k, v) in [
        ("PRODUCT", "Běžecké boty"),
        ("URL", "https://shop.example.com/products/bezecke-boty"),
        ("CATEGORY", "Obuv"),
        ("VENDOR", "Alfa"),
        ("TAGS", "sport, běh"),
        ("IMGURL", "https://cdn.shop.cz/a.jpg"),
        ("PUBLISHED_AT", "2026-01-01T00:00:00+01:00"),
        ("DESCRIPTION", "Lehké boty na běh."),
    ] {
        assert_eq!(r[k], v);
    }
}
#[test]
fn missing_shared_fields() {
    let mut p = product();
    p["images"] = json!([]);
    assert_eq!(product_fields(&p, STORE)["IMGURL"], "");
    p.as_object_mut().unwrap().remove("images");
    assert_eq!(product_fields(&p, STORE)["IMGURL"], "");
    p["tags"] = Value::Null;
    assert_eq!(product_fields(&p, STORE)["TAGS"], "");
    assert_eq!(
        product_fields(&json!({"title":"X"}), STORE)["URL"],
        format!("{STORE}/products/")
    );
}
#[test]
fn variants() {
    let rows = variant_rows(&product(), STORE);
    assert_eq!(rows.len(), 2);
    for r in &rows {
        for k in COLUMNS {
            if k != "IMAGE_FILENAME" {
                assert!(r.contains_key(k), "{k}");
            }
        }
        assert_eq!(r["VENDOR"], "Alfa");
        assert_eq!(r["IMGURL"], "https://cdn.shop.cz/a.jpg");
    }
    assert_eq!(rows[0]["PRODUCTNO"], "A-42");
    assert_eq!(rows[1]["PRODUCTNO"], "A-43");
    assert_eq!(rows[0]["STOCK"], "Yes");
    assert_eq!(rows[1]["STOCK"], "No");
    assert!(rows[1]["COMPARE_AT_PRICE"].is_null());
    assert_eq!(cell(&rows[1]["COMPARE_AT_PRICE"]), "");
    assert_eq!(rows[0]["WEIGHT"], 350);
    assert_eq!(rows[0]["TAXABLE"], "Yes");
    assert_eq!(rows[1]["TAXABLE"], "No");
}
#[test]
fn missing_variants() {
    let mut p = product();
    p["variants"] = json!([]);
    assert!(variant_rows(&p, STORE).is_empty());
    p.as_object_mut().unwrap().remove("variants");
    assert!(variant_rows(&p, STORE).is_empty());
    p["variants"] = json!([{"title":"Default Title","price":"100"}]);
    assert_eq!(variant_rows(&p, STORE)[0]["PRODUCT"], "Běžecké boty");
    p["variants"] = json!([{"title":"42"}]);
    let r = variant_rows(&p, STORE).remove(0);
    assert_eq!(r["PRICE"], "");
    assert_eq!(r["STOCK"], "No");
}
#[test]
fn csv_encoding_and_quoting() {
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let file = temp.path().join("out.csv");
    let mut r = Row::new();
    r.insert("PRODUCT".into(), json!("Žluť, \"kůň\"\nX"));
    shopify_scraper::write_csv(&file, &[r]).unwrap();
    let data = std::fs::read(&file).unwrap();
    let text = String::from_utf8(data).unwrap();
    assert!(text.starts_with(&format!("{}\r\n", COLUMNS.join(","))));
    assert!(text.contains("\"Žluť, \"\"kůň\"\"\nX\""));
}
