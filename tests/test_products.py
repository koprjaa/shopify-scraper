"""Tests for turning a Shopify product into CSV rows. No network.

The product below is the shape /products.json serves: a product with shared
fields and a list of variants that each become one row.
"""

import pytest

from products import (
    COLUMNS,
    product_fields,
    remove_html_tags,
    variant_name,
    variant_rows,
    yes_no,
)

STORE = "https://shop.example.com"

PRODUCT = {
    "title": "Běžecké boty",
    "handle": "bezecke-boty",
    "product_type": "Obuv",
    "vendor": "Alfa",
    "tags": ["sport", "běh"],
    "published_at": "2026-01-01T00:00:00+01:00",
    "created_at": "2025-12-01T00:00:00+01:00",
    "updated_at": "2026-02-01T00:00:00+01:00",
    "body_html": "<p>Lehké boty <strong>na běh</strong>.</p>",
    "images": [{"src": "https://cdn.shop.cz/a.jpg"}, {"src": "https://cdn.shop.cz/b.jpg"}],
    "variants": [
        {
            "title": "42", "price": "1990.00", "compare_at_price": "2490.00",
            "available": True, "sku": "A-42", "barcode": "859000000001",
            "weight": 350, "weight_unit": "g",
            "requires_shipping": True, "taxable": True,
        },
        {
            "title": "43", "price": "1990.00", "compare_at_price": None,
            "available": False, "sku": "A-43", "barcode": "",
            "weight": 360, "weight_unit": "g",
            "requires_shipping": True, "taxable": False,
        },
    ],
}


# --- remove_html_tags -------------------------------------------------------


def test_html_is_stripped_from_a_description():
    assert remove_html_tags("<p>Lehké boty <strong>na běh</strong>.</p>") == "Lehké boty na běh."


@pytest.mark.parametrize("value", ["", None])
def test_an_empty_description_gives_an_empty_string(value):
    assert remove_html_tags(value) == ""


def test_a_description_without_markup_is_unchanged():
    assert remove_html_tags("Plain text") == "Plain text"


def test_a_description_that_is_not_a_string_is_coerced():
    """Shopify sends null for a product with no description."""
    assert remove_html_tags(123) == "123"


# --- yes_no -----------------------------------------------------------------


@pytest.mark.parametrize("value", [True, 1, "text", [1]])
def test_a_truthy_flag_reads_yes(value):
    assert yes_no(value) == "Yes"


@pytest.mark.parametrize("value", [False, 0, "", None, []])
def test_a_falsy_or_missing_flag_reads_no(value):
    assert yes_no(value) == "No"


# --- variant_name -----------------------------------------------------------


def test_a_real_variant_name_is_appended():
    assert variant_name("Boty", "42") == "Boty - 42"


def test_the_shopify_placeholder_is_dropped():
    """A product with no real variants gets one called "Default Title"."""
    assert variant_name("Boty", "Default Title") == "Boty"


@pytest.mark.parametrize("variant_title", ["", None])
def test_a_missing_variant_name_leaves_the_product_name(variant_title):
    assert variant_name("Boty", variant_title) == "Boty"


def test_a_variant_name_with_slashes_survives():
    assert variant_name("Boty", "Black / L") == "Boty - Black / L"


# --- product_fields ---------------------------------------------------------


def test_the_shared_fields_are_read_off_the_product():
    fields = product_fields(PRODUCT, STORE)
    assert fields["PRODUCT"] == "Běžecké boty"
    assert fields["URL"] == f"{STORE}/products/bezecke-boty"
    assert fields["CATEGORY"] == "Obuv"
    assert fields["VENDOR"] == "Alfa"


def test_the_tags_become_one_string():
    assert product_fields(PRODUCT, STORE)["TAGS"] == "sport, běh"


def test_the_first_image_is_the_one_recorded():
    assert product_fields(PRODUCT, STORE)["IMGURL"] == "https://cdn.shop.cz/a.jpg"


def test_a_product_with_no_images_records_none():
    assert product_fields({**PRODUCT, "images": []}, STORE)["IMGURL"] == ""


def test_a_product_missing_images_entirely_does_not_fail():
    product = {k: v for k, v in PRODUCT.items() if k != "images"}
    assert product_fields(product, STORE)["IMGURL"] == ""


def test_a_product_with_no_tags_gives_an_empty_string():
    assert product_fields({**PRODUCT, "tags": None}, STORE)["TAGS"] == ""


def test_a_product_missing_every_optional_field_still_yields_a_row():
    assert product_fields({"title": "X"}, STORE)["URL"] == f"{STORE}/products/"


# --- variant_rows -----------------------------------------------------------


def test_one_row_comes_out_per_variant():
    assert len(variant_rows(PRODUCT, STORE)) == 2


def test_every_row_carries_the_full_column_set():
    """A row missing a column shifts every later value in the CSV."""
    for row in variant_rows(PRODUCT, STORE):
        for column in COLUMNS:
            if column != "IMAGE_FILENAME":  # filled in after the image downloads
                assert column in row, column


def test_the_variant_fields_land_on_their_own_row():
    first, second = variant_rows(PRODUCT, STORE)
    assert first["PRODUCTNO"] == "A-42"
    assert second["PRODUCTNO"] == "A-43"
    assert first["STOCK"] == "Yes"
    assert second["STOCK"] == "No"


def test_the_shared_fields_repeat_on_every_row():
    rows = variant_rows(PRODUCT, STORE)
    assert {row["VENDOR"] for row in rows} == {"Alfa"}
    assert {row["IMGURL"] for row in rows} == {"https://cdn.shop.cz/a.jpg"}


def test_a_missing_compare_price_becomes_an_empty_cell():
    assert variant_rows(PRODUCT, STORE)[1]["COMPARE_AT_PRICE"] is None


def test_a_product_with_no_variants_yields_no_rows():
    assert variant_rows({**PRODUCT, "variants": []}, STORE) == []


def test_a_product_missing_the_variants_key_yields_no_rows():
    product = {k: v for k, v in PRODUCT.items() if k != "variants"}
    assert variant_rows(product, STORE) == []


def test_a_single_variant_product_is_named_after_the_product():
    product = {**PRODUCT, "variants": [{"title": "Default Title", "price": "100"}]}
    assert variant_rows(product, STORE)[0]["PRODUCT"] == "Běžecké boty"


def test_a_variant_missing_every_optional_field_still_yields_a_row():
    product = {**PRODUCT, "variants": [{"title": "42"}]}
    row = variant_rows(product, STORE)[0]
    assert row["PRICE"] == ""
    assert row["STOCK"] == "No"
