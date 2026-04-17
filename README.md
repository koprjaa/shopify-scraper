# shopify-scraper

**No-API-key scraper that pulls product data from any Shopify store via the public `/products.json` endpoint, downloads all images in parallel, and dumps a 20-column CSV.**

![python](https://img.shields.io/badge/python-3.10+-3776AB?style=flat-square&logo=python&logoColor=white)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)
![status](https://img.shields.io/badge/status-active-22863A?style=flat-square)
![requests](https://img.shields.io/badge/requests-2.32-000?style=flat-square)
![threadpool](https://img.shields.io/badge/ThreadPoolExecutor-parallel%20IO-blueviolet?style=flat-square)

Every Shopify store exposes its catalogue at conventional JSON endpoints — no OAuth, no app install, no rate-limit headaches. This tool walks them, paginates through every product, unpacks variants, downloads images concurrently, and writes the whole thing to CSV.

## Run it

```bash
python shopify_scraper.py https://shop.example.com
```

One-shot for the entire catalogue. For a specific collection (or several):

```bash
python shopify_scraper.py https://shop.example.com -c mens-shoes womens-shoes
```

Output lands in `shopify_exports/<domain>_export_<timestamp>/`:

```
shopify_exports/
└── shop.example.com_export_20260417_151856/
    ├── images/
    │   ├── product1_variant1.jpg
    │   └── product1_variant2.jpg
    ├── shop_example_com_shopify_products.csv
    └── shopify_scraper.log
```

## CSV columns

`PRODUCT`, `URL`, `PRICE`, `COMPARE_AT_PRICE`, `STOCK`, `CATEGORY`, `PRODUCTNO`, `BARCODE`, `WEIGHT`, `WEIGHT_UNIT`, `REQUIRES_SHIPPING`, `TAXABLE`, `VENDOR`, `TAGS`, `PUBLISHED_AT`, `CREATED_AT`, `UPDATED_AT`, `DESCRIPTION`, `IMGURL`, `IMAGE_FILENAME`

## Flags

| flag | default | effect |
|------|---------|--------|
| positional `url` | required | Shopify store URL |
| `-c, --collections` | all | specific collection handles |
| `-o, --output-folder` | `shopify_exports` | output parent |
| `-f, --csv-filename` | `shopify_products.csv` | CSV file name |
| `-l, --log-filename` | `shopify_scraper.log` | log file name |
| `-r, --max-retries` | 3 | retries per failed request |
| `-d, --retry-delay` | 180s | base retry delay |
| `-v, --verbosity` | 2 (info) | 1=error, 2=info, 3=debug |

## Install

```bash
uv venv
uv pip install -r requirements.txt
```

## Notes

- Uses `ThreadPoolExecutor` (no thread count cap) for image downloads — respectful sequential fetches for product pages, aggressive parallel for static image URLs
- Retry delay is deliberately long (180s) because Shopify IP-bans rather than returning 429 on bulk pulls
- Works on any Shopify store — `.myshopify.com` subdomains, custom domains, whatever `/products.json` resolves against

## License

[MIT](LICENSE)
