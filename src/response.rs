// Project: shopify-scraper
// File: response.rs
// Description: Decode JSON responses with the original client's Unicode detection.
// Author: Jan Alexandr Kopřiva
// License: MIT
use crate::Result;
use serde_json::Value;

pub fn decode_json(bytes: &[u8], content_type: &str) -> Result<Value> {
    // Detect UTF-32 before UTF-16: their little-endian BOM prefixes overlap.
    let utf32 = if bytes.starts_with(&[0xff, 0xfe, 0, 0]) {
        Some((true, &bytes[4..]))
    } else if bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        Some((false, &bytes[4..]))
    } else if bytes.len() >= 4 && bytes[1..4] == [0, 0, 0] {
        Some((true, bytes))
    } else if bytes.len() >= 4 && bytes[..3] == [0, 0, 0] {
        Some((false, bytes))
    } else {
        None
    };
    if let Some((little, bytes)) = utf32 {
        if bytes.len() % 4 != 0 {
            return Err("incomplete UTF-32 response".into());
        }
        let text = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|chunk| {
                let word = [chunk[0], chunk[1], chunk[2], chunk[3]];
                char::from_u32(if little {
                    u32::from_le_bytes(word)
                } else {
                    u32::from_be_bytes(word)
                })
                .ok_or("invalid UTF-32 character")
            })
            .collect::<std::result::Result<String, _>>()?;
        return Ok(serde_json::from_str(&text)?);
    }
    let charset = content_type.split(';').skip(1).find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        key.eq_ignore_ascii_case("charset")
            .then(|| value.trim().trim_matches(['\'', '"']))
    });
    if charset.is_some_and(|s| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "iso-8859-1" | "latin-1" | "latin1"
        )
    }) || (charset.is_none() && content_type.to_ascii_lowercase().contains("text"))
    {
        let text: String = bytes.iter().map(|&b| char::from(b)).collect();
        return Ok(serde_json::from_str(&text)?);
    }
    let encoding = charset
        .and_then(|s| encoding_rs::Encoding::for_label(s.as_bytes()))
        .or_else(|| {
            if bytes.len() >= 4 && bytes[1] == 0 && bytes[3] == 0 {
                Some(encoding_rs::UTF_16LE)
            } else if bytes.len() >= 4 && bytes[0] == 0 && bytes[2] == 0 {
                Some(encoding_rs::UTF_16BE)
            } else {
                None
            }
        })
        .unwrap_or(encoding_rs::UTF_8);
    let (text, _, _) = encoding.decode(bytes);
    Ok(serde_json::from_str(text.trim_start_matches('\u{feff}'))?)
}
