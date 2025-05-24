# Shopify Product Scraper

A robust Python tool for extracting product data from Shopify stores. Efficiently fetches product information, images, and metadata, saving them in structured CSV format.

## Features

- Comprehensive product data extraction
- High-quality image downloads
- Structured CSV output
- Collection-specific scraping
- Configurable retry mechanism
- Progress tracking
- Detailed logging
- Error handling
- Concurrent processing

## Quick Start

```sh
# Clone repository
git clone https://github.com/koprjaa/shopify_scraper.git
cd shopify_scraper

# Install dependencies
pip install -r requirements.txt

# Run scraper
python shopify_scraper.py <shopify_store_url> [options]
```

## Usage

### Command-line Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `url` | Shopify store URL | Required |
| `-c, --collections` | Collection handles | None |
| `-o, --output-folder` | Output path | `shopify_exports` |
| `-f, --csv-filename` | CSV filename | `shopify_products.csv` |
| `-l, --log-filename` | Log filename | `shopify_scraper.log` |
| `-r, --max-retries` | Max retries | 3 |
| `-d, --retry-delay` | Retry delay (s) | 180 |
| `-v, --verbosity` | Log level (1-3) | 2 |

### Examples

```sh
# Basic usage
python shopify_scraper.py https://example.myshopify.com

# Specific collections
python shopify_scraper.py https://example.myshopify.com -c collection1 collection2

# Custom output
python shopify_scraper.py https://example.myshopify.com -o custom_folder -f products.csv

# Debug mode
python shopify_scraper.py https://example.myshopify.com -v 3 -r 5 -d 300
```