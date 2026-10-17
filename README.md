# shopify-scraper

Reads product data from any Shopify store through the public JSON endpoints, downloads images in parallel, and writes a 20 column CSV. No API key is needed.

![rust](https://img.shields.io/badge/rust-stable-DEA584?style=flat-square&logo=rust&logoColor=white)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)
[![ci](https://github.com/koprjaa/shopify-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/shopify-scraper/actions/workflows/ci.yml)

## Install

Install a stable Rust toolchain, then run:

```bash
cargo install --path .
```

## Use

Read all listed collections:

```bash
shopify-scraper https://shop.example.com
```

Read one or more collections, including handles absent from the listing:

```bash
shopify-scraper https://shop.example.com -c mens-shoes womens-shoes
```

Output goes to `shopify_exports/<store>_export_<UTC timestamp>/`:

```text
shopify_exports/
  shop_export_20260417_151856/
    images/
      product.jpg
    shop_shopify_products.csv
    shopify_scraper.log
```

The store name is the first hostname label after removing `www.`. CSV columns, in order:

`PRODUCT`, `URL`, `PRICE`, `COMPARE_AT_PRICE`, `STOCK`, `CATEGORY`, `PRODUCTNO`, `BARCODE`, `WEIGHT`, `WEIGHT_UNIT`, `REQUIRES_SHIPPING`, `TAXABLE`, `VENDOR`, `TAGS`, `PUBLISHED_AT`, `CREATED_AT`, `UPDATED_AT`, `DESCRIPTION`, `IMGURL`, `IMAGE_FILENAME`.

CSV uses UTF-8 without BOM, commas, doubled double quotes and CRLF record endings. Null cells are empty. Flags read `Yes`/`No`. HTML tags are removed without decoding entities. No JSON, XLSX or XML files are produced.

## Options

| Flag | Default | Effect |
|---|---|---|
| `-c, --collections` | all listed | One or more collection handles. |
| `-o, --output-folder` | `shopify_exports` | Parent folder. |
| `-f, --csv-filename` | `shopify_products.csv` | CSV basename, prefixed with store name. |
| `-l, --log-filename` | `shopify_scraper.log` | Append log filename. |
| `-r, --max-retries` | 3 | Total attempts per catalogue request. |
| `-d, --retry-delay` | 5 | Initial retry delay in seconds, doubled between failures. |
| `-v, --verbosity` | 2 | 1 error, 2 info, 3 debug. |

Help exits 0; invalid arguments exit 2; runtime errors exit 1; completed exports exit 0. Nonpositive attempt counts fail without making a request. Negative delays fail only when a retry requires sleeping.

## How it works

`src/products.rs` creates one row per variant. `Default Title` and empty variant names use the product name alone. Products without variants yield no rows. Products repeated across collections are retained at their first occurrence, with every variant from that occurrence.

Without explicit handles, collection listing pages and their product pages are fetched lazily in order. Explicit handles bypass listing. A store with no listed collections produces a header-only CSV; there is no automatic fallback to the whole-store product endpoint.

Catalogue requests use the original legacy Chrome User-Agent, three attempts and delays of 5 and 10 seconds by default. No sleep follows the last failure. Images use `python-requests/2.33.0`, no retry, and a worker limit of `min(32, available CPUs + 4)`, using the original executor formula on Python 3.10–3.12. CPU availability follows Rust’s platform detection and may respect container quotas that older Python versions ignored. Requests have no timeout or additional rate limit. The CSV is written before downloads and rewritten with successful image filenames afterwards. Failed image requests leave the filename empty and do not fail the export; file errors do.

There is no application configuration file or application-specific environment variable. Like requests, optional Basic authentication is read from `NETRC` or the home directory’s `.netrc`/`_netrc` file; credentials stay outside the source. HTTP proxy environment variables supported by reqwest remain available. TLS uses rustls and bundled public trust roots. `REQUESTS_CA_BUNDLE`, with `CURL_CA_BUNDLE` as fallback, can select a PEM certificate bundle.

## Limits

The public endpoints expose no metafields or private fields. Those require the Shopify Admin API. This tool does not use that API.

## Changes from the Python version

- The executable is `shopify-scraper`; install with Cargo. Python modules and their mutable global Settings API are replaced by Rust types. Help/usage and argument error formatting come from clap; option names, defaults and exit codes remain the same. Unique long-option abbreviations remain accepted.
- Fixed loss of variants: the old export deduplicated each row by product URL and kept only the first variant. Deduplication now applies to product occurrences, keeping all variants. Counts and CSV row totals consequently include all variants; duplicate debug messages now describe repeated product occurrences rather than discarded variants.
- Fixed same-second output overwrites: existing export directories receive `_2`, `_3`, etc. Atomic directory creation also separates simultaneous runs.
- Fixed concurrent image overwrites: a URL shared by products/variants is downloaded once and its filename applied to every corresponding row. Different URLs with the same basename receive deterministic suffixes (`photo_2.jpg`, etc.). Invalid/empty basenames in full exports use `image`. Image counts reflect distinct URLs. Debug product-image messages are emitted after workers finish; completion progress remains based on finished downloads.
- Logging is scoped to each scraper, preventing handler accumulation on repeated library calls. Log files are explicitly UTF-8 on every platform. Python tracebacks and requests/urllib3 internal debug output are replaced by Rust error descriptions; network warning prefixes and application messages retain their original wording. Native exception details, TLS/connection diagnostics, compression negotiation and HTTP header order can differ by HTTP implementation. The image User-Agent is pinned to the original dependency's minimum version rather than varying with the installed requests package.
- The worker-limit formula is unchanged, but Rust CPU detection may respect container quotas that older Python versions ignored.
- TLS uses bundled rustls trust roots instead of requests/certifi. Requests-specific proxy edge cases can differ. Custom certificate directories are not supported (use a PEM bundle); invalid certificate paths fail during client initialization rather than inside the retry loop. Malformed netrc files are ignored, as in requests; permission checks and exotic netrc lexical cases may differ. No credentials are embedded. The 30-redirect limit is preserved; connections are reused rather than creating a new requests session per call.
- Malformed JSON field types are handled permissively: non-array variants/images/tags/products/collections act as empty; non-string tags are stringified instead of raising TypeError; a missing first image src is empty instead of KeyError. Scalar stringification preserves null/boolean/number behavior; nonempty arrays/objects use JSON notation instead of Python repr. Expected Shopify payloads are unaffected. Invalid JSON responses are still retried.
- Negative or overflowing retry sleeps return a runtime error rather than a Python exception/traceback. Filesystem errors exit 1 without a Python traceback. Timestamps retain second precision; elapsed execution time excludes directory/logger/client initialization. CLI retry counts and delays are bounded by signed 32-bit and 64-bit integers, respectively, rather than Python’s unbounded integers.
- The old README's 180-second default, full-domain folder names and uncapped-image-thread claim were inconsistent with its code. This README documents the actual 5-second default, short store names and bounded pool.

## Development

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Tests cover every original product/request scenario, CLI exit codes and files, CSV encoding/quoting, transient failures, invalid JSON, image downloads and variant/image regression cases. HTTP tests use an in-process loopback fixture server and never contact live stores. CI runs on Ubuntu and Windows.

Dependencies: reqwest supplies blocking HTTP, rustls, compression, JSON and proxy support; serde_json represents Shopify payloads; csv handles compatible quoting; clap parses the CLI; chrono formats UTC output folders/local log timestamps; regex preserves description stripping semantics. `src/auth.rs` handles optional netrc credentials. The only development dependency, tempfile, isolates test output. The local mock server uses the standard library.

[SEMANTICS.md](SEMANTICS.md) records the original behavior and edge cases. [REPORT.md](REPORT.md) records verification.

## License

[MIT](LICENSE)
