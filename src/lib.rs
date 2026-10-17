// Project: shopify-scraper
// File: lib.rs
// Description: Requests, pagination, logging, image downloads and CSV export.
// Author: Jan Alexandr Kopřiva
// License: MIT

pub mod auth;
pub mod products;
pub mod response;
use chrono::{Local, Utc};
use clap::Parser;
use products::{cell, variant_rows, Row, COLUMNS};
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_9_3) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/35.0.1916.47 Safari/537.36";
#[derive(Parser, Debug, Clone)]
#[command(
    about = "Shopify product scraper",
    disable_version_flag = true,
    infer_long_args = true,
    args_override_self = true
)]
pub struct Settings {
    #[arg(help = "Shopify store URL")]
    pub url: String,
    #[arg(short, long, num_args = 1.., action = clap::ArgAction::Set)]
    pub collections: Vec<String>,
    #[arg(short = 'o', long, default_value = "shopify_exports")]
    pub output_folder: PathBuf,
    #[arg(short = 'f', long, default_value = "shopify_products.csv")]
    pub csv_filename: String,
    #[arg(short = 'l', long, default_value = "shopify_scraper.log")]
    pub log_filename: String,
    #[arg(short = 'r', long, default_value_t = 3, allow_hyphen_values = true)]
    pub max_retries: i32,
    #[arg(short = 'd', long, default_value_t = 5, allow_hyphen_values = true)]
    pub retry_delay: i64,
    #[arg(short = 'v', long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..=3))]
    pub verbosity: u8,
}
pub struct Logger {
    file: Mutex<File>,
    verbosity: u8,
}
impl Logger {
    pub fn new(path: &Path, verbosity: u8) -> Result<Self> {
        Ok(Self {
            file: Mutex::new(OpenOptions::new().create(true).append(true).open(path)?),
            verbosity,
        })
    }
    pub fn log(&self, level: &str, msg: &str) {
        if self.verbosity == 1 && level != "ERROR" || self.verbosity < 3 && level == "DEBUG" {
            return;
        }
        let line = format!(
            "{} - {level} - {msg}",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        eprintln!("{line}");
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "{line}");
        }
    }
}
pub fn store_name_from_url(url: &str) -> String {
    // Keep the URL's hostname spelling rather than WHATWG/IDNA normalization for output names.
    let authority = if let Some(rest) = url.strip_prefix("//") {
        rest
    } else if let Some((_, rest)) = url.split_once("://") {
        rest
    } else {
        return "store".into();
    };
    let netloc = authority.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = netloc.rsplit('@').next().unwrap_or("");
    let host = if let Some(rest) = host_port.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        host_port.split(':').next().unwrap_or("")
    }
    .to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if host.is_empty() {
        "store".into()
    } else {
        host.split('.').next().unwrap_or("store").into()
    }
}
pub fn retry_delay(base: i64, attempt: u32) -> Result<Duration> {
    if base == 0 {
        return Ok(Duration::ZERO);
    }
    let seconds = base
        .checked_mul(2_i64.checked_pow(attempt).ok_or("retry delay overflow")?)
        .ok_or("retry delay overflow")?;
    Ok(Duration::from_secs(
        seconds
            .try_into()
            .map_err(|_| "sleep length must be non-negative")?,
    ))
}
pub struct Scraper {
    pub settings: Settings,
    client: Client,
    pub logger: Logger,
}
impl Scraper {
    pub fn new(settings: Settings, logger: Logger) -> Result<Self> {
        Ok(Self {
            settings,
            client: http_client()?,
            logger,
        })
    }
    pub fn make_request(&self, url: &str) -> Result<Value> {
        self.make_request_with_sleep(url, std::thread::sleep)
    }
    pub fn make_request_with_sleep(
        &self,
        url: &str,
        mut sleep: impl FnMut(Duration),
    ) -> Result<Value> {
        for attempt in 0..self.settings.max_retries {
            self.logger.log(
                "DEBUG",
                &format!(
                    "Attempting request to {url} (Attempt {}/{})",
                    attempt + 1,
                    self.settings.max_retries
                ),
            );
            let answer = (|| -> Result<Value> {
                let response = authenticated(&self.client, url)
                    .header("User-Agent", USER_AGENT)
                    .header("Accept", "*/*")
                    .header("Accept-Encoding", "gzip, deflate")
                    .header("Connection", "keep-alive")
                    .send()?
                    .error_for_status()?;
                let content_type = response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("")
                    .to_owned();
                response::decode_json(&response.bytes()?, &content_type)
            })();
            match answer {
                Ok(v) => {
                    self.logger
                        .log("DEBUG", &format!("Request to {url} successful"));
                    return Ok(v);
                }
                Err(e) => {
                    let mut msg = format!(
                        "Request failed (attempt {}/{}): {e}.",
                        attempt + 1,
                        self.settings.max_retries
                    );
                    if attempt < self.settings.max_retries - 1 {
                        let delay = retry_delay(self.settings.retry_delay, attempt as u32)?;
                        msg.push_str(&format!(" Retrying in {} seconds...", delay.as_secs()));
                        self.logger.log("WARNING", &msg);
                        sleep(delay);
                    } else {
                        self.logger.log("WARNING", &msg);
                    }
                }
            }
        }
        Err(format!(
            "Failed to retrieve data from {url} after {} attempts.",
            self.settings.max_retries
        )
        .into())
    }
    pub fn get_page(&self, url: &str, page: usize, handle: Option<&str>) -> Result<Vec<Value>> {
        let endpoint = match handle.filter(|h| !h.is_empty()) {
            Some(h) => format!("{url}/collections/{h}/products.json?page={page}"),
            None => format!("{url}/products.json?page={page}"),
        };
        let data = self.make_request(&endpoint)?;
        if !data.is_object() {
            return Err("product response must be an object".into());
        }
        match data.get("products") {
            None | Some(Value::Null) => Ok(Vec::new()),
            Some(Value::Array(products)) => {
                if products.iter().any(|p| !p.is_object()) {
                    return Err("products must contain objects".into());
                }
                Ok(products.clone())
            }
            _ => Err("products must be an array".into()),
        }
    }
    fn collection(
        &self,
        handle: &str,
        rows: &mut Vec<Row>,
        seen: &mut HashSet<String>,
    ) -> Result<()> {
        self.logger.log(
            "INFO",
            &format!("Extracting products from collection: {handle}"),
        );
        let mut count = 0;
        for page in 1.. {
            let products = self.get_page(&self.settings.url, page, Some(handle))?;
            if products.is_empty() {
                break;
            }
            for p in products {
                let variants = variant_rows(&p, &self.settings.url);
                count += variants.len();
                // Deduplicate collection occurrences, retaining every variant of the first occurrence.
                if variants.is_empty() {
                    continue;
                }
                let url = cell(&variants[0]["URL"]);
                if seen.insert(url.clone()) {
                    rows.extend(variants);
                } else {
                    self.logger
                        .log("DEBUG", &format!("Duplicate product found: {url}"));
                }
            }
            self.logger.log(
                "INFO",
                &format!("Extracted {count} products from collection '{handle}'"),
            );
        }
        Ok(())
    }
    pub fn collect(&self) -> Result<Vec<Row>> {
        let mut rows = Vec::new();
        let mut seen = HashSet::new();
        if !self.settings.collections.is_empty() {
            for h in &self.settings.collections {
                self.collection(h, &mut rows, &mut seen)?;
            }
        } else {
            for page in 1.. {
                let data = self.make_request(&format!(
                    "{}/collections.json?page={page}",
                    self.settings.url
                ))?;
                if !data.is_object() {
                    return Err("collection response must be an object".into());
                }
                let cols = match data.get("collections") {
                    None | Some(Value::Null) => break,
                    Some(Value::Array(cols)) if cols.is_empty() => break,
                    Some(Value::Array(cols)) => cols,
                    _ => return Err("collections must be an array".into()),
                };
                for col in cols {
                    let h = col["handle"].as_str().ok_or("collection missing handle")?;
                    self.collection(h, &mut rows, &mut seen)?;
                }
            }
        }
        Ok(rows)
    }
    pub fn download_image(&self, url: &str, folder: &Path) -> Result<Option<String>> {
        self.download_named(url, folder, None)
    }
    fn download_named(
        &self,
        url: &str,
        folder: &Path,
        assigned: Option<&str>,
    ) -> Result<Option<String>> {
        if url.is_empty() {
            return Ok(None);
        }
        let response = authenticated(&self.client, url)
            .header("User-Agent", "python-requests/2.33.0")
            .header("Accept", "*/*")
            .header("Accept-Encoding", "gzip, deflate")
            .header("Connection", "keep-alive")
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.bytes());
        match response {
            Ok(bytes) => {
                let parsed = reqwest::Url::parse(url)?;
                let filename = assigned.unwrap_or_else(|| {
                    parsed
                        .path()
                        .trim_end_matches('/')
                        .rsplit('/')
                        .next()
                        .unwrap_or("")
                });
                if filename.is_empty() || filename == "." || filename == ".." {
                    return Err("invalid image filename".into());
                }
                fs::write(folder.join(filename), bytes)?;
                self.logger
                    .log("DEBUG", &format!("Downloaded image: {filename}"));
                Ok(Some(filename.into()))
            }
            Err(e) => {
                self.logger
                    .log("ERROR", &format!("Error downloading image from {url}\n{e}"));
                Ok(None)
            }
        }
    }
    pub fn run(&self, folder: &Path) -> Result<()> {
        let start = Instant::now();
        let images = folder.join("images");
        self.logger.log(
            "INFO",
            &format!("Created output folder: {}", folder.display()),
        );
        self.logger.log(
            "INFO",
            &format!("Created images folder: {}", images.display()),
        );
        self.logger.log("INFO", "Extracting products...");
        let mut rows = self.collect()?;
        self.logger.log(
            "INFO",
            &format!("Total unique products found: {}", rows.len()),
        );
        let csv = folder.join(format!(
            "{}_{}",
            store_name_from_url(&self.settings.url),
            self.settings.csv_filename
        ));
        write_csv(&csv, &rows)?;
        self.logger.log(
            "INFO",
            &format!("Unique product data saved to {}", csv.display()),
        );
        // Download each distinct URL once, preventing concurrent writes to the same image.
        let mut urls = Vec::new();
        let mut seen = HashSet::new();
        for row in &rows {
            let u = cell(&row["IMGURL"]);
            if !u.is_empty() && seen.insert(u.clone()) {
                urls.push(u);
            }
        }
        let names = image_names(&urls);
        let total = urls.len();
        self.logger.log(
            "INFO",
            &format!("Starting download of {total} unique product images"),
        );
        let next = AtomicUsize::new(0);
        let completed = Mutex::new(0usize);
        let results = Mutex::new(Vec::new());
        let workers = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            .saturating_add(4)
            .min(32)
            .min(total);
        std::thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= total {
                        break;
                    }
                    let result = self.download_named(&urls[i], &images, Some(&names[i]));
                    results.lock().unwrap().push((i, result));
                    let mut n = completed.lock().unwrap();
                    *n += 1;
                    if (*n).is_multiple_of(10) || *n == total {
                        self.logger
                            .log("INFO", &format!("Downloaded {n}/{total} images."));
                    }
                });
            }
        });
        for (i, result) in results.into_inner().unwrap() {
            if let Some(filename) = result? {
                for row in &mut rows {
                    if cell(&row["IMGURL"]) == urls[i] {
                        row.insert("IMAGE_FILENAME".into(), filename.clone().into());
                        self.logger.log(
                            "DEBUG",
                            &format!("Downloaded image for product: {}", cell(&row["PRODUCT"])),
                        );
                    }
                }
            }
        }
        self.logger.log("INFO", "Image download completed.");
        write_csv(&csv, &rows)?;
        self.logger.log(
            "INFO",
            &format!("Updated unique product data saved to {}", csv.display()),
        );
        self.logger
            .log("INFO", &format!("Images saved to {}", images.display()));
        self.logger.log(
            "INFO",
            &format!(
                "Total execution time: {:.2} seconds",
                start.elapsed().as_secs_f64()
            ),
        );
        self.logger
            .log("INFO", "Product extraction completed successfully");
        Ok(())
    }
}
pub fn write_csv(path: &Path, rows: &[Row]) -> Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_path(path)?;
    writer.write_record(COLUMNS)?;
    for row in rows {
        writer.write_record(
            COLUMNS
                .iter()
                .map(|k| row.get(*k).map(cell).unwrap_or_default()),
        )?;
    }
    writer.flush()?;
    Ok(())
}
pub fn output_folder(s: &Settings) -> Result<PathBuf> {
    fs::create_dir_all(&s.output_folder)?;
    let name = format!(
        "{}_export_{}",
        store_name_from_url(&s.url),
        Utc::now().format("%Y%m%d_%H%M%S")
    );
    let mut suffix = 1;
    loop {
        let path = s.output_folder.join(if suffix == 1 {
            name.clone()
        } else {
            format!("{name}_{suffix}")
        });
        match fs::create_dir(&path) {
            Ok(()) => {
                fs::create_dir(path.join("images"))?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                suffix += 1;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

// Reserve all names before starting workers so colliding basenames cannot overwrite each other.
pub fn image_names(urls: &[String]) -> Vec<String> {
    let mut used = HashSet::new();
    urls.iter()
        .map(|url| {
            let base = reqwest::Url::parse(url)
                .ok()
                .and_then(|u| {
                    u.path()
                        .trim_end_matches('/')
                        .rsplit('/')
                        .next()
                        .map(str::to_owned)
                })
                .filter(|s| !s.is_empty() && s != "." && s != "..")
                .unwrap_or("image".into());
            let mut name = base.clone();
            let mut suffix = 2;
            while !used.insert(name.clone()) {
                let path = Path::new(&base);
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(&base);
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| format!(".{s}"))
                    .unwrap_or_default();
                name = format!("{stem}_{suffix}{ext}");
                suffix += 1;
            }
            name
        })
        .collect()
}

pub fn http_client() -> Result<Client> {
    http_client_with_timeout(Duration::from_secs(30))
}

pub fn http_client_with_timeout(timeout: Duration) -> Result<Client> {
    // Bound both stalled connections and stalled response bodies, including image workers.
    let mut builder = Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(30));
    let bundle = std::env::var_os("REQUESTS_CA_BUNDLE")
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var_os("CURL_CA_BUNDLE").filter(|v| !v.is_empty()));
    if let Some(path) = bundle {
        let bytes = fs::read(path)?;
        builder = builder.tls_built_in_root_certs(false);
        for cert in reqwest::Certificate::from_pem_bundle(&bytes)? {
            builder = builder.add_root_certificate(cert);
        }
    }
    Ok(builder.build()?)
}

fn authenticated(client: &Client, url: &str) -> reqwest::blocking::RequestBuilder {
    let request = client.get(url);
    if let Some((username, password)) = auth::credentials(url) {
        request.basic_auth(username, Some(password))
    } else {
        request
    }
}
