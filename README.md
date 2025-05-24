# Shopify Product Scraper

A powerful Python tool for extracting product information from Shopify stores. This scraper efficiently fetches product data including prices, stock status, variants, and images, saving them to CSV files and downloading product images to a specified folder.

## Features

- 🚀 Extract comprehensive product information from Shopify stores
- 📸 Download high-quality product images
- 📊 Save structured data to CSV format
- 🎯 Support for specific collection scraping
- 🔄 Configurable retry mechanism for reliability
- 📈 Progress tracking with tqdm
- 📝 Detailed logging for debugging
- 🛡️ Error handling and recovery
- ⚡ Efficient concurrent processing
- 🔍 Support for pagination and large catalogs

## Installation

1. Clone the repository:
```sh
git clone https://github.com/koprjaa/shopify_scraper.git
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

| Argument | Description | Default |
|----------|-------------|---------|
| `url` | The URL of the Shopify store to scrape | Required |
| `-c, --collections` | List of collection handles to extract | None |
| `-o, --output-folder` | Output folder path | `shopify_exports` |
| `-f, --csv-filename` | CSV file name | `shopify_products.csv` |
| `-l, --log-filename` | Log file name | `shopify_scraper.log` |
| `-r, --max-retries` | Maximum number of retries for failed requests | 3 |
| `-d, --retry-delay` | Retry delay in seconds | 180 |
| `-v, --verbosity` | Verbosity level (1=error, 2=info, 3=debug) | 2 |

### Examples

1. Extract all products from a Shopify store:
```sh
python shopify_scraper.py https://example.myshopify.com
```

2. Extract products from specific collections:
```sh
python shopify_scraper.py https://example.myshopify.com -c collection1 collection2
```

3. Extract products with custom output settings:
```sh
python shopify_scraper.py https://example.myshopify.com -o custom_folder -f products.csv -l scraper.log
```

4. Extract products with increased verbosity and retry settings:
```sh
python shopify_scraper.py https://example.myshopify.com -v 3 -r 5 -d 300
```

## Output

The scraper generates the following files in the specified output folder:

### CSV File
- Product title
- Product description
- Price information
- Stock status
- Variant details
- Product URL
- Image URLs
- Collection information
- Additional metadata

### Image Files
- High-resolution product images
- Organized in a dedicated images subfolder
- Named according to product identifiers

### Log File
- Detailed operation information
- Error tracking
- Performance metrics
- Request/response data (in debug mode)

## Error Handling

The scraper includes robust error handling:
- Automatic retry mechanism for failed requests
- Configurable retry count and delay
- Detailed error logging
- Graceful failure recovery

## Performance

- Efficient concurrent processing
- Memory-optimized image handling
- Progress tracking for long-running operations
- Configurable verbosity levels

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

- Koprjaa (https://github.com/koprjaa)

## Acknowledgments

- Thanks to all contributors who have helped improve this project
- Inspired by the need for efficient Shopify store data extraction
