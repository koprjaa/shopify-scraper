
# Shopify Product Scraper

## Introduction
The Shopify Product Scraper is a tool designed to extract product information from Shopify stores. It fetches product data, including details like price, stock status, and images, and saves this information into a CSV file. The tool also downloads product images to a specified folder.

## Table of Contents
- [Introduction](#introduction)
- [Table of Contents](#table-of-contents)
- [Installation](#installation)
- [Usage](#usage)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)
- [Contributors](#contributors)
- [License](#license)

## Installation
To use this scraper, you need Python 3.x installed on your system. You can install the required dependencies using pip:

```sh
pip install -r requirements.txt
```

## Usage
Run the scraper using the command line:

```sh
python scraper.py <shopify_store_url> [options]
```

### Command-line Arguments
- `url`: The URL of the Shopify store to scrape.
- `-c, --collections`: A list of collection handles to extract (optional).
- `-o, --output-folder`: The output folder path (default: `shopify_exports`).
- `-f, --csv-filename`: The CSV file name (default: `shopify_products.csv`).
- `-l, --log-filename`: The log file name (default: `shopify_scraper.log`).
- `-r, --max-retries`: Maximum number of retries for failed requests (default: 3).
- `-d, --retry-delay`: Retry delay in seconds (default: 180).
- `-v, --verbosity`: Verbosity level (1=error, 2=info, 3=debug; default: 2).


## Examples
Extract products from a Shopify store and save to the default output folder:

```sh
python scraper.py https://example.myshopify.com
```

Extract products from specific collections:

```sh
python scraper.py https://example.myshopify.com -c collection1 collection2
```

## Contributors
- Koprjaa(https://github.com/koprjaa)

## License
This project is licensed under the MIT License.
