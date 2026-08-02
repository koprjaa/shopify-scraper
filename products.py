#
# Project: shopify-scraper
# File:    products.py
#
# Description:
# Turns a Shopify product into one CSV row per variant.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

"""Turning a Shopify product into the rows that go into the CSV.

Every Shopify store serves the same JSON at /products.json, so the shape of a
product is fixed and worth pinning down. Nothing here makes a request.
"""

import re

HTML_TAG_RE = re.compile(r"<[^<]+?>")

# Columns in the output, in order.
COLUMNS = [
    "PRODUCT",
    "URL",
    "PRICE",
    "COMPARE_AT_PRICE",
    "STOCK",
    "CATEGORY",
    "PRODUCTNO",
    "BARCODE",
    "WEIGHT",
    "WEIGHT_UNIT",
    "REQUIRES_SHIPPING",
    "TAXABLE",
    "VENDOR",
    "TAGS",
    "PUBLISHED_AT",
    "CREATED_AT",
    "UPDATED_AT",
    "DESCRIPTION",
    "IMGURL",
    "IMAGE_FILENAME",
]


def remove_html_tags(text) -> str:
    """Plain text of a product description."""
    return HTML_TAG_RE.sub("", str(text or ""))


def yes_no(value) -> str:
    """A flag as the spreadsheet spells it."""
    return "Yes" if value else "No"


def product_fields(product: dict, store_url: str) -> dict:
    """The fields every variant of a product shares."""
    images = product.get("images") or []
    return {
        "PRODUCT": product.get("title", ""),
        "URL": f"{store_url}/products/{product.get('handle', '')}",
        "CATEGORY": product.get("product_type", ""),
        "VENDOR": product.get("vendor", ""),
        "TAGS": ", ".join(product.get("tags") or []),
        "PUBLISHED_AT": product.get("published_at", ""),
        "CREATED_AT": product.get("created_at", ""),
        "UPDATED_AT": product.get("updated_at", ""),
        "DESCRIPTION": remove_html_tags(product.get("body_html")),
        "IMGURL": images[0]["src"] if images else "",
    }


def variant_name(product_title: str, variant_title: str) -> str:
    """Name of one variant.

    A product with a single variant names it "Default Title", which says
    nothing, so the product name is used on its own.
    """
    if not variant_title or variant_title == "Default Title":
        return product_title
    return f"{product_title} - {variant_title}".strip(" -")


def variant_rows(product: dict, store_url: str) -> list[dict]:
    """One row per variant. A product with no variants yields nothing."""
    shared = product_fields(product, store_url)
    return [
        {
            **shared,
            "PRODUCT": variant_name(shared["PRODUCT"], variant.get("title", "")),
            "PRICE": variant.get("price", ""),
            "COMPARE_AT_PRICE": variant.get("compare_at_price", ""),
            "STOCK": yes_no(variant.get("available")),
            "PRODUCTNO": variant.get("sku", ""),
            "BARCODE": variant.get("barcode", ""),
            "WEIGHT": variant.get("weight", ""),
            "WEIGHT_UNIT": variant.get("weight_unit", ""),
            "REQUIRES_SHIPPING": yes_no(variant.get("requires_shipping")),
            "TAXABLE": yes_no(variant.get("taxable")),
        }
        for variant in product.get("variants") or []
    ]
