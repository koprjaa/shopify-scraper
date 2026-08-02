#
# Project: shopify-scraper
# File:    test_requests.py
#
# Description:
# Tests that a store which never answers is reported as a failure, not as an empty catalogue.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

"""A refused store used to look exactly like an empty one.

make_request returned None after exhausting its retries, and every caller reads
an empty result as the end of pagination. A store answering 429 to everything
therefore produced a header-only CSV and the line "Product extraction completed
successfully". These tests keep the two apart.
"""

import sys
from pathlib import Path

import pytest
import requests

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import shopify_scraper as s

STORE = "https://shop.example.com"


class Response:
    def __init__(self, payload=None, error=None):
        self._payload = payload
        self._error = error

    def raise_for_status(self):
        if self._error:
            raise self._error

    def json(self):
        return self._payload


@pytest.fixture(autouse=True)
def _no_sleep(monkeypatch):
    monkeypatch.setattr(s.time, "sleep", lambda _: None)


def always_fails(monkeypatch, error=None):
    error = error or requests.HTTPError("429 Too Many Requests")
    calls = []

    def get(url, **kwargs):
        calls.append(url)
        return Response(error=error)

    monkeypatch.setattr(s.requests, "get", get)
    return calls


def answers(monkeypatch, payloads):
    """Answer each successive request with the next payload."""
    queue = list(payloads)

    def get(url, **kwargs):
        return Response(payload=queue.pop(0) if queue else {})

    monkeypatch.setattr(s.requests, "get", get)


# --- make_request ------------------------------------------------------------


def test_a_store_that_never_answers_raises(monkeypatch):
    always_fails(monkeypatch)
    with pytest.raises(s.ShopifyRequestError):
        s.make_request(f"{STORE}/products.json")


def test_the_error_names_the_url_that_failed(monkeypatch):
    always_fails(monkeypatch)
    with pytest.raises(s.ShopifyRequestError, match=r"products\.json"):
        s.make_request(f"{STORE}/products.json")


def test_every_retry_is_used_before_giving_up(monkeypatch):
    calls = always_fails(monkeypatch)
    with pytest.raises(s.ShopifyRequestError):
        s.make_request(f"{STORE}/products.json")
    assert len(calls) == s.Settings.MAX_RETRIES


def test_a_successful_request_returns_the_payload(monkeypatch):
    answers(monkeypatch, [{"products": [{"id": 1}]}])
    assert s.make_request(f"{STORE}/products.json") == {"products": [{"id": 1}]}


# --- get_page ----------------------------------------------------------------


def test_an_empty_page_is_not_an_error(monkeypatch):
    """The end of pagination and a refused request must not look the same."""
    answers(monkeypatch, [{"products": []}])
    assert s.get_page(STORE, 1) == []


def test_a_refused_page_raises_rather_than_reading_as_the_last_page(monkeypatch):
    always_fails(monkeypatch)
    with pytest.raises(s.ShopifyRequestError):
        s.get_page(STORE, 1)


def test_a_page_missing_the_products_key_reads_as_empty(monkeypatch):
    answers(monkeypatch, [{}])
    assert s.get_page(STORE, 1) == []


def test_a_collection_page_uses_the_collection_url(monkeypatch):
    seen = []

    def get(url, **kwargs):
        seen.append(url)
        return Response(payload={"products": []})

    monkeypatch.setattr(s.requests, "get", get)
    s.get_page(STORE, 1, "socks")
    assert seen == [f"{STORE}/collections/socks/products.json?page=1"]


# --- get_page_collections ----------------------------------------------------


def test_collection_listing_stops_on_an_empty_page(monkeypatch):
    answers(monkeypatch, [{"collections": [{"handle": "a"}]}, {"collections": []}])
    assert [c["handle"] for c in s.get_page_collections(STORE)] == ["a"]


def test_a_refused_collection_listing_raises(monkeypatch):
    always_fails(monkeypatch)
    with pytest.raises(s.ShopifyRequestError):
        list(s.get_page_collections(STORE))


# --- which endpoints a run actually calls ------------------------------------

PRODUCT = {
    "title": "Sock",
    "handle": "sock",
    "images": [],
    "variants": [{"title": "Default Title", "price": "10.00"}],
}


def record_urls(monkeypatch, tmp_path):
    """Answer every request plausibly and remember which URLs were asked for."""
    seen = []

    def get(url, **kwargs):
        seen.append(url)
        if "/collections.json" in url:
            page = url.rsplit("page=", 1)[-1]
            return Response(payload={"collections": [{"handle": "listed"}] if page == "1" else []})
        if "/products.json" in url:
            page = url.rsplit("page=", 1)[-1]
            return Response(payload={"products": [PRODUCT] if page == "1" else []})
        return Response(payload={})

    monkeypatch.setattr(s.requests, "get", get)
    monkeypatch.setattr(s.Settings, "OUTPUT_FOLDER", str(tmp_path))
    return seen


def test_a_named_collection_is_fetched_without_listing_every_collection(monkeypatch, tmp_path):
    """/collections.json omits smart collections, so filtering against it dropped
    a valid handle and returned nothing. It is also a request per page for a
    listing the caller already told us it does not need."""
    seen = record_urls(monkeypatch, tmp_path)
    s.extract_products(STORE, ["socks"])
    assert not any("/collections.json" in u for u in seen)
    assert any("/collections/socks/products.json" in u for u in seen)


def test_a_named_collection_that_is_not_in_the_listing_still_yields_products(monkeypatch, tmp_path):
    record_urls(monkeypatch, tmp_path)
    s.extract_products(STORE, ["socks"])
    csv_files = list(Path(tmp_path).rglob("*.csv"))
    assert csv_files, "no CSV written"
    assert len(csv_files[0].read_text(encoding="utf-8").strip().splitlines()) == 2


def test_without_a_named_collection_the_listing_is_used(monkeypatch, tmp_path):
    seen = record_urls(monkeypatch, tmp_path)
    s.extract_products(STORE, None)
    assert any("/collections.json" in u for u in seen)
    assert any("/collections/listed/products.json" in u for u in seen)


def test_several_named_collections_are_each_fetched(monkeypatch, tmp_path):
    seen = record_urls(monkeypatch, tmp_path)
    s.extract_products(STORE, ["socks", "shoes"])
    for handle in ("socks", "shoes"):
        assert any(f"/collections/{handle}/products.json" in u for u in seen)


# --- the store name that goes into every output path -------------------------


@pytest.mark.parametrize(
    ("url", "expected"),
    [
        ("https://www.allbirds.com", "allbirds"),
        ("https://kith.com", "kith"),
        ("https://demo.myshopify.com", "demo"),
        ("https://www.shop.example.co.uk", "shop"),
        ("http://www.example.com/collections/all", "example"),
        ("https://WWW.Example.COM", "example"),
    ],
)
def test_the_store_name_comes_from_the_host(url, expected):
    """It used to be the first label of the hostname, so every www store was
    called "www" and two of them overwrote each other's export folder."""
    assert s.store_name_from_url(url) == expected


def test_two_www_stores_no_longer_collide():
    assert s.store_name_from_url("https://www.allbirds.com") != s.store_name_from_url(
        "https://www.gymshark.com"
    )


def test_a_url_without_a_host_still_gives_a_usable_name():
    assert s.store_name_from_url("not-a-url") == "store"


# --- backoff -----------------------------------------------------------------


def test_the_delay_doubles_and_none_is_slept_after_the_last_attempt(monkeypatch):
    slept = []
    monkeypatch.setattr(s.time, "sleep", slept.append)
    monkeypatch.setattr(s.Settings, "RETRY_DELAY", 5)
    monkeypatch.setattr(s.Settings, "MAX_RETRIES", 3)
    always_fails(monkeypatch)

    with pytest.raises(s.ShopifyRequestError):
        s.make_request(f"{STORE}/products.json")

    assert slept == [5, 10], "expected one sleep between attempts, doubling, none at the end"


def test_a_successful_request_sleeps_not_at_all(monkeypatch):
    slept = []
    monkeypatch.setattr(s.time, "sleep", slept.append)
    answers(monkeypatch, [{"products": []}])
    s.make_request(f"{STORE}/products.json")
    assert slept == []
