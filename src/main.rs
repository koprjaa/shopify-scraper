// Project: shopify-scraper
// File: main.rs
// Description: Command line entry point.
// Author: Jan Alexandr Kopřiva
// License: MIT
use clap::Parser;
use shopify_scraper::{output_folder, Logger, Scraper, Settings};
fn main() {
    let settings = Settings::parse();
    let setup = (|| -> shopify_scraper::Result<_> {
        let folder = output_folder(&settings)?;
        let logger = Logger::new(&folder.join(&settings.log_filename), settings.verbosity)?;
        let scraper = Scraper::new(settings, logger)?;
        Ok((folder, scraper))
    })();
    let (folder, scraper) = match setup {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "{} - ERROR - {error}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            );
            std::process::exit(1);
        }
    };
    if let Err(error) = scraper.run(&folder) {
        scraper.logger.log("ERROR", &error.to_string());
        std::process::exit(1);
    }
}
