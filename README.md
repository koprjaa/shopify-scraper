# shopify-scraper

Reads product data from any Shopify store through the public `/products.json` endpoint, downloads the images in parallel, and writes a 20 column CSV. No API key is needed.

![python](https://img.shields.io/badge/python-3.10+-3776AB?style=flat-square&logo=python&logoColor=white)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)
![status](https://img.shields.io/badge/status-active-22863A?style=flat-square)

Every Shopify store serves its catalogue at fixed JSON paths. There is no OAuth step and no app install. The tool pages through every product, unpacks the variants, downloads the images, and writes the result to CSV.

## Install

```bash
uv venv
uv pip install -r requirements.txt
```

## Use

Read the whole catalogue:

```bash
python shopify_scraper.py https://shop.example.com
```

Read one or more collections:

```bash
python shopify_scraper.py https://shop.example.com -c mens-shoes womens-shoes
```

Output goes to `shopify_exports/<domain>_export_<timestamp>/`:

```
shopify_exports/
  shop.example.com_export_20260417_151856/
    images/
      product1_variant1.jpg
      product1_variant2.jpg
    shop_example_com_shopify_products.csv
    shopify_scraper.log
```

The CSV holds these columns: `PRODUCT`, `URL`, `PRICE`, `COMPARE_AT_PRICE`, `STOCK`, `CATEGORY`, `PRODUCTNO`, `BARCODE`, `WEIGHT`, `WEIGHT_UNIT`, `REQUIRES_SHIPPING`, `TAXABLE`, `VENDOR`, `TAGS`, `PUBLISHED_AT`, `CREATED_AT`, `UPDATED_AT`, `DESCRIPTION`, `IMGURL`, `IMAGE_FILENAME`.

## Options

| Flag | Default | Effect |
|---|---|---|
| `-c, --collections` | all | Specific collection handles. |
| `-o, --output-folder` | `shopify_exports` | Parent folder for the output. |
| `-f, --csv-filename` | `shopify_products.csv` | Name of the CSV file. |
| `-l, --log-filename` | `shopify_scraper.log` | Name of the log file. |
| `-r, --max-retries` | 3 | Retries per failed request. |
| `-d, --retry-delay` | 180 | Base retry delay in seconds. |
| `-v, --verbosity` | 2 | 1 for error, 2 for info, 3 for debug. |

## How it works

Product pages load one after another. Image downloads run in parallel through a `ThreadPoolExecutor` with no thread cap, because image URLs point at static storage.

The retry delay defaults to 180 seconds. Shopify bans the IP address on bulk reads instead of returning a 429 response.

The tool works against `.myshopify.com` subdomains and custom domains. Any host that answers `/products.json` works.

## Limits

- The public endpoint exposes no metafields and no private fields. Those need the Shopify Admin API with an access token.
- The repository history contains a large export of scraped product images from three real stores. Clone with `--depth 1` if you only want the code.
- No tests.

## License

[MIT](LICENSE)
