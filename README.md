# shopify-scraper

Exports the product feed of any Shopify store.

[![CI](https://github.com/koprjaa/shopify-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/shopify-scraper/actions/workflows/ci.yml)
[![MIT license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)](LICENSE)

## What it does

Reads public product data without an API key and writes a CSV with 20 columns. Each variant gets a row with prices, availability, SKU and other product fields. Products repeated across collections appear only once, with all their variants.

Downloads each product's first image into `images/` and records successful downloads in `IMAGE_FILENAME`. Failed image requests leave that field empty.

## Install

Install a stable Rust toolchain, then run from the repository directory:

```bash
cargo install --path .
```

No environment variables are required.

## Use

Replace the example URL with your store URL. To export all listed collections:

```bash
shopify-scraper https://shop.example.com
```

To select collection handles and change the output parent folder:

```bash
shopify-scraper https://shop.example.com -c mens-shoes womens-shoes -o exports
```

Selected handles bypass the collection listing. Without selected handles, a store with no listed collections produces a CSV containing only the header.

By default, each run creates `shopify_exports/<store>_export_<UTC timestamp>/` with `<store>_shopify_products.csv`, `shopify_scraper.log` and `images/`. The store name is the first hostname label after removing `www.`.

Use `shopify-scraper --help` for more options.

## License

[MIT](LICENSE)
