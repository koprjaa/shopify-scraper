// Project: shopify-scraper
// File: auth.rs
// Description: Preserve requests' optional environment and netrc authentication.
// Author: Jan Alexandr Kopřiva
// License: MIT
use std::{fs, path::PathBuf};

pub fn credentials(url: &str) -> Option<(String, String)> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let paths = if let Some(path) = std::env::var_os("NETRC") {
        vec![PathBuf::from(path)]
    } else {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        vec![
            PathBuf::from(&home).join(".netrc"),
            PathBuf::from(home).join("_netrc"),
        ]
    };
    for path in paths {
        if path.exists() {
            return parse(&fs::read_to_string(path).ok()?, host);
        }
    }
    None
}

pub fn parse(text: &str, host: &str) -> Option<(String, String)> {
    // netrc macros are shell snippets; requests never executes them.
    let mut filtered = String::new();
    let mut in_macro = false;
    for line in text.lines() {
        if in_macro {
            if line.trim().is_empty() {
                in_macro = false;
            }
            continue;
        }
        if line.trim_start().starts_with("macdef ") {
            in_macro = true;
            continue;
        }
        filtered.push_str(line);
        filtered.push('\n');
    }
    let mut tokens = Vec::new();
    let mut chars = filtered.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if c == '#' {
            for c in chars.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            continue;
        }
        let mut token = String::new();
        if c == '"' {
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '"' {
                    closed = true;
                    break;
                }
                if c == '\\' {
                    token.push(chars.next()?);
                } else {
                    token.push(c);
                }
            }
            if !closed {
                return None;
            }
        } else {
            if c == '\\' {
                token.push(chars.next()?);
            } else {
                token.push(c);
            }
            while let Some(c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                let c = chars.next()?;
                if c == '\\' {
                    token.push(chars.next()?);
                } else {
                    token.push(c);
                }
            }
        }
        tokens.push(token);
    }
    let mut index = 0;
    let mut fallback = None;
    let mut matched = None;
    while index < tokens.len() {
        let machine = match tokens[index].as_str() {
            "machine" => {
                index += 1;
                let name = tokens.get(index)?.clone();
                index += 1;
                name
            }
            "default" => {
                index += 1;
                "default".to_owned()
            }
            _ => return None,
        };
        let mut login = None;
        let mut account = None;
        let mut password = String::new();
        while index < tokens.len() && tokens[index] != "machine" && tokens[index] != "default" {
            let key = &tokens[index];
            index += 1;
            let value = tokens.get(index)?.clone();
            index += 1;
            match key.as_str() {
                "login" | "user" => login = Some(value),
                "account" => account = Some(value),
                "password" => password = value,
                _ => return None,
            }
        }
        let entry = (
            login
                .filter(|s| !s.is_empty())
                .or(account)
                .unwrap_or_default(),
            password,
        );
        if machine == host {
            matched = Some(entry);
        } else if machine == "default" {
            fallback = Some(entry);
        }
    }
    matched.or(fallback)
}
