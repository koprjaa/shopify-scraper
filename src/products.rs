// Project: shopify-scraper
// File: products.rs
// Description: Turns a Shopify product into CSV rows.
// Author: Jan Alexandr Kopřiva
// License: MIT

use serde_json::Value;
use std::{collections::BTreeMap, sync::OnceLock};
pub type Row = BTreeMap<String, Value>;
pub const COLUMNS: [&str; 20] = [
    "PRODUCT",
    "URL",
    "PRICE",
    "COMPARE_AT_PRICE",
    "STOCK",
    "CATEGORY",
    "PRODUCTNO",
    "BARCODE",
    "WEIGHT",
    "WEIGHT_UNIT",
    "REQUIRES_SHIPPING",
    "TAXABLE",
    "VENDOR",
    "TAGS",
    "PUBLISHED_AT",
    "CREATED_AT",
    "UPDATED_AT",
    "DESCRIPTION",
    "IMGURL",
    "IMAGE_FILENAME",
];
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}
pub fn py_string(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}
pub fn cell(v: &Value) -> String {
    if v.is_null() {
        String::new()
    } else {
        py_string(v)
    }
}
pub fn remove_html_tags(v: &Value) -> String {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"<[^<]+?>").unwrap())
        .replace_all(
            &if truthy(v) {
                py_string(v)
            } else {
                String::new()
            },
            "",
        )
        .into_owned()
}
pub fn yes_no(v: &Value) -> &'static str {
    if truthy(v) {
        "Yes"
    } else {
        "No"
    }
}
fn field(v: &Value, k: &str) -> Value {
    v.get(k).cloned().unwrap_or(Value::String(String::new()))
}
pub fn variant_name(title: &Value, variant: &Value) -> Value {
    if !truthy(variant) || variant == "Default Title" {
        title.clone()
    } else {
        Value::String(
            format!("{} - {}", py_string(title), py_string(variant))
                .trim_matches([' ', '-'])
                .into(),
        )
    }
}
pub fn product_fields(p: &Value, url: &str) -> Row {
    let mut r = Row::new();
    for (column, key) in [
        ("PRODUCT", "title"),
        ("CATEGORY", "product_type"),
        ("VENDOR", "vendor"),
        ("PUBLISHED_AT", "published_at"),
        ("CREATED_AT", "created_at"),
        ("UPDATED_AT", "updated_at"),
    ] {
        r.insert(column.into(), field(p, key));
    }
    r.insert(
        "URL".into(),
        format!("{url}/products/{}", py_string(&field(p, "handle"))).into(),
    );
    r.insert(
        "TAGS".into(),
        p["tags"]
            .as_array()
            .map(|a| a.iter().map(py_string).collect::<Vec<_>>().join(", "))
            .unwrap_or_default()
            .into(),
    );
    r.insert(
        "DESCRIPTION".into(),
        remove_html_tags(&p["body_html"]).into(),
    );
    r.insert(
        "IMGURL".into(),
        p["images"]
            .get(0)
            .map(|i| field(i, "src"))
            .unwrap_or("".into()),
    );
    r
}
pub fn variant_rows(p: &Value, url: &str) -> Vec<Row> {
    let shared = product_fields(p, url);
    p["variants"]
        .as_array()
        .map(|vs| {
            vs.iter()
                .map(|v| {
                    let mut r = shared.clone();
                    r.insert(
                        "PRODUCT".into(),
                        variant_name(&shared["PRODUCT"], &field(v, "title")),
                    );
                    for (col, key) in [
                        ("PRICE", "price"),
                        ("COMPARE_AT_PRICE", "compare_at_price"),
                        ("PRODUCTNO", "sku"),
                        ("BARCODE", "barcode"),
                        ("WEIGHT", "weight"),
                        ("WEIGHT_UNIT", "weight_unit"),
                    ] {
                        r.insert(col.into(), field(v, key));
                    }
                    for (col, key) in [
                        ("STOCK", "available"),
                        ("REQUIRES_SHIPPING", "requires_shipping"),
                        ("TAXABLE", "taxable"),
                    ] {
                        r.insert(col.into(), yes_no(&v[key]).into());
                    }
                    r
                })
                .collect()
        })
        .unwrap_or_default()
}
