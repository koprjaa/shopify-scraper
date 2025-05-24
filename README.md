# Shopify Product Scraper

A Python tool for extracting product information from Shopify stores. This scraper fetches product data including prices, stock status, and images, saving them to CSV files and downloading product images to a specified folder.

## Features

- Extract product information from Shopify stores
- Download product images
- Save data to CSV format
- Support for specific collection scraping
- Configurable retry mechanism
- Progress tracking with tqdm
- Detailed logging

## Installation

1. Clone the repository:
```sh
git clone https://github.com/yourusername/shopify_scraper.git
cd shopify_scraper
```

2. Install the required dependencies:
```sh
pip install -r requirements.txt
```

## Usage

Run the scraper using the command line:

```sh
python shopify_scraper.py <shopify_store_url> [options]
```

### Command-line Arguments

- `url`: The URL of the Shopify store to scrape
- `-c, --collections`: List of collection handles to extract (optional)
- `-o, --output-folder`: Output folder path (default: `shopify_exports`)
- `-f, --csv-filename`: CSV file name (default: `shopify_products.csv`)
- `-l, --log-filename`: Log file name (default: `shopify_scraper.log`)
- `-r, --max-retries`: Maximum number of retries for failed requests (default: 3)
- `-d, --retry-delay`: Retry delay in seconds (default: 180)
- `-v, --verbosity`: Verbosity level (1=error, 2=info, 3=debug; default: 2)

### Examples

Extract all products from a Shopify store:
```sh
python shopify_scraper.py https://example.myshopify.com
```

Extract products from specific collections:
```sh
python shopify_scraper.py https://example.myshopify.com -c collection1 collection2
```

Extract products with custom output settings:
```sh
python shopify_scraper.py https://example.myshopify.com -o custom_folder -f products.csv -l scraper.log
```

## Output

The scraper generates:
- A CSV file containing product information
- A folder with downloaded product images
- A log file with detailed operation information

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

- Koprjaa (https://github.com/koprjaa)
