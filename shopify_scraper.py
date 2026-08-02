#
# Project: shopify-scraper
# File:    shopify_scraper.py
#
# Description:
# Reads products and collections from a Shopify store and writes them to CSV along with their images.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

import argparse
import csv
import logging
import sys
import time
from collections import OrderedDict
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path
from typing import ClassVar
from urllib.parse import urlparse

import requests

from products import variant_rows

logger = logging.getLogger(__name__)


class ShopifyRequestError(RuntimeError):
    """A request the store never answered, after every retry."""


class Settings:
    """
    A class to store the application settings.
    Modify this class to change the default settings for the scraper.
    """

    # Output folder for the extracted data
    OUTPUT_FOLDER = "shopify_exports"

    # CSV file name for the exported product data
    CSV_FILENAME = "shopify_products.csv"

    # Log file name for the scraper logs
    LOG_FILENAME = "shopify_scraper.log"

    # Maximum number of retries for failed requests
    MAX_RETRIES = 3

    # Seconds before the first retry. Doubles on each further attempt, so three
    # attempts wait 5 + 10 rather than 180 + 180.
    RETRY_DELAY = 5

    # Logging level (1=error, 2=info, 3=debug)
    LOG_LEVEL = 2

    # User agent string for the HTTP requests
    USER_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_9_3) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/35.0.1916.47 Safari/537.36"

    # Define the headers for the CSV file
    HEADERS: ClassVar[list[str]] = [
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


def make_request(url, headers=None):
    """
    Make a GET request with retry logic.

    Args:
    url (str): The URL to make the request to.
    headers (dict, optional): Headers to include in the request. Defaults to None.

    Returns:
    dict: The JSON response from the server, or None if the request failed.
    """
    if headers is None:
        headers = {"User-Agent": Settings.USER_AGENT}

    for attempt in range(Settings.MAX_RETRIES):
        try:
            logger.debug(
                f"Attempting request to {url} (Attempt {attempt + 1}/{Settings.MAX_RETRIES})"
            )
            response = requests.get(url, headers=headers)
            response.raise_for_status()
            logger.debug(f"Request to {url} successful")
            return response.json()
        except requests.RequestException as e:
            if attempt == Settings.MAX_RETRIES - 1:
                logger.warning(f"Request failed (attempt {attempt + 1}/{Settings.MAX_RETRIES}): {e}.")
                break
            # Back off rather than sleeping the same delay every time. A flat 180
            # seconds meant three attempts against a store that does not answer
            # took nine minutes and looked like a hang. There is also no reason
            # to sleep after the last attempt, which the old loop always did.
            delay = Settings.RETRY_DELAY * (2**attempt)
            logger.warning(
                f"Request failed (attempt {attempt + 1}/{Settings.MAX_RETRIES}): {e}. "
                f"Retrying in {delay} seconds..."
            )
            time.sleep(delay)

    # Raise rather than return None. The callers treat an empty result as the end
    # of pagination, so returning None on failure made a store that refused every
    # request look like a store that had run out of pages: the run then wrote a
    # header-only CSV and reported success.
    raise ShopifyRequestError(
        f"Failed to retrieve data from {url} after {Settings.MAX_RETRIES} attempts."
    )


def store_name_from_url(url):
    """Short name for a store, used for the output folder and the CSV name.

    Taking the first label of the hostname gave "www" for every store that uses
    it, so www.allbirds.com and www.example.com both wrote to www_export_* and
    the second run overwrote the first. The www prefix is dropped before the
    name is taken.

    Args:
    url (str): The base URL of the Shopify store.

    Returns:
    str: The store name, or "store" when the URL carries no host.
    """
    host = (urlparse(url).hostname or "").removeprefix("www.")
    return host.split(".")[0] if host else "store"


def get_page(url, page, collection_handle=None):
    """
    Fetch a page of products from a Shopify store.

    Args:
    url (str): The base URL of the Shopify store.
    page (int): The page number to fetch.
    collection_handle (str, optional): The handle of the collection to fetch products from. Defaults to None.

    Returns:
    list: A list of product dictionaries, or an empty list if the request failed.
    """
    full_url = f"{url}/products.json?page={page}"
    if collection_handle:
        full_url = f"{url}/collections/{collection_handle}/products.json?page={page}"

    return make_request(full_url).get("products", [])


def get_page_collections(url):
    """
    Generator to fetch all collections from a Shopify store.

    Args:
    url (str): The base URL of the Shopify store.

    Yields:
    dict: A dictionary containing information about each collection.
    """
    page = 1
    while True:
        full_url = f"{url}/collections.json?page={page}"
        data = make_request(full_url)
        if not data.get("collections"):
            break
        yield from data["collections"]
        page += 1


def extract_products_collection(url, col):
    """
    Generator to extract all products from a collection.

    Args:
    url (str): The base URL of the Shopify store.
    col (str): The handle of the collection to extract products from.

    Yields:

    dict: A dictionary containing information about each product variant.
    """
    page = 1
    products_count = 0
    while True:
        products = get_page(url, page, col)
        if not products:
            break

        for product in products:
            rows = variant_rows(product, url)
            yield from rows
            products_count += len(rows)

        page += 1
        logger.info(f"Extracted {products_count} products from collection '{col}'")


def download_image(url, folder_path):
    """
    Download an image from a URL and save it to a specified folder.

    Args:
    url (str): The URL of the image to download.
    folder_path (str): The path to the folder where the image should be saved.

    Returns:
    str: The filename of the downloaded image, or None if the download failed.
    """
    if not url:
        return None

    try:
        response = requests.get(url)
        response.raise_for_status()
        filename = Path(urlparse(url).path).name
        filepath = Path(folder_path) / filename
        with filepath.open("wb") as f:
            f.write(response.content)
        logger.debug(f"Downloaded image: {filename}")
        return filename
    except requests.RequestException:
        logger.exception("Error downloading image from %s", url)
    return None


def setup_output_folders(store_name, output_folder):
    """
    Create the output folder and log file for the extraction.

    Args:
    store_name (str): The name of the Shopify store.
    output_folder (str): The path to the output folder.

    Returns:
    tuple: The path to the output folder, images folder, and log file.
    """
    timestamp = datetime.now(tz=timezone.utc).strftime("%Y%m%d_%H%M%S")
    folder_name = f"{store_name}_export_{timestamp}"
    output_folder = str(Path(output_folder) / folder_name)
    Path(output_folder).mkdir(parents=True, exist_ok=True)

    images_folder = str(Path(output_folder) / "images")
    Path(images_folder).mkdir(parents=True, exist_ok=True)

    log_file = str(Path(output_folder) / Settings.LOG_FILENAME)

    return output_folder, images_folder, log_file


def extract_products(url, collections=None):
    """
    Main function to extract products and download images from a Shopify store.

    Args:
    url (str): The base URL of the Shopify store.
    collections (list, optional): A list of collection handles to extract products from. Defaults to None.
    """
    total_start_time = time.time()

    # Create output folders and log file
    store_name = store_name_from_url(url)
    output_folder, images_folder, log_file = setup_output_folders(
        store_name, Settings.OUTPUT_FOLDER
    )

    # Configure the root logger, so the module logger above propagates into it.
    root_logger = logging.getLogger()
    root_logger.setLevel(Settings.LOG_LEVEL)
    formatter = logging.Formatter(
        "%(asctime)s - %(levelname)s - %(message)s", datefmt="%Y-%m-%d %H:%M:%S"
    )

    file_handler = logging.FileHandler(log_file)
    file_handler.setFormatter(formatter)
    root_logger.addHandler(file_handler)

    console_handler = logging.StreamHandler()
    console_handler.setFormatter(formatter)
    root_logger.addHandler(console_handler)

    logger.info(f"Created output folder: {output_folder}")
    logger.info(f"Created images folder: {images_folder}")

    output_file = str(Path(output_folder) / f"{store_name}_{Settings.CSV_FILENAME}")

    # Use OrderedDict to maintain insertion order and remove duplicates
    unique_products = OrderedDict()

    # Extract all products
    logger.info("Extracting products...")

    # A handle the caller named is fetched straight from its own endpoint. Walking
    # the whole listing and filtering afterwards costs a request per page of
    # collections, and /collections.json omits smart collections, so a perfectly
    # valid handle was silently skipped and the run finished with nothing.
    handles = collections or (col["handle"] for col in get_page_collections(url))

    for handle in handles:
        logger.info(f"Extracting products from collection: {handle}")
        for product in extract_products_collection(url, handle):
            # Use product URL as the unique identifier
            if product["URL"] not in unique_products:
                unique_products[product["URL"]] = product
            else:
                logger.debug(f"Duplicate product found: {product['URL']}")

    logger.info(f"Total unique products found: {len(unique_products)}")

    # Write unique products to CSV
    with Path(output_file).open("w", newline="", encoding="utf-8") as csvfile:
        writer = csv.DictWriter(csvfile, fieldnames=Settings.HEADERS)
        writer.writeheader()
        for product in unique_products.values():
            writer.writerow(product)

    logger.info(f"Unique product data saved to {output_file}")

    # Download images for unique products
    products_to_download = [
        product for product in unique_products.values() if product["IMGURL"]
    ]
    total_images = len(products_to_download)
    logger.info(f"Starting download of {total_images} unique product images")

    with ThreadPoolExecutor() as executor:
        future_to_product = {
            executor.submit(download_image, product["IMGURL"], images_folder): product
            for product in products_to_download
        }

        for i, future in enumerate(as_completed(future_to_product), 1):
            product = future_to_product[future]
            filename = future.result()
            if filename:
                product["IMAGE_FILENAME"] = filename
                logger.debug(f"Downloaded image for product: {product['PRODUCT']}")

            if i % 10 == 0 or i == total_images:
                logger.info(f"Downloaded {i}/{total_images} images.")

    logger.info("Image download completed.")

    # Update the CSV with image filenames
    with Path(output_file).open("w", newline="", encoding="utf-8") as csvfile:
        writer = csv.DictWriter(csvfile, fieldnames=Settings.HEADERS)
        writer.writeheader()
        for product in unique_products.values():
            writer.writerow(product)

    logger.info(f"Updated unique product data saved to {output_file}")

    total_time = time.time() - total_start_time
    logger.info(f"Images saved to {images_folder}")
    logger.info(f"Total execution time: {total_time:.2f} seconds")
    logger.info("Product extraction completed successfully")


if __name__ == "__main__":
    # Set up the command-line argument parser
    parser = argparse.ArgumentParser(description="Shopify product scraper")
    parser.add_argument("url", help="Shopify store URL")
    parser.add_argument(
        "-c", "--collections", nargs="+", help="List of collection handles to extract"
    )
    parser.add_argument(
        "-o",
        "--output-folder",
        default=Settings.OUTPUT_FOLDER,
        help="Output folder path",
    )
    parser.add_argument(
        "-f", "--csv-filename", default=Settings.CSV_FILENAME, help="CSV file name"
    )
    parser.add_argument(
        "-l", "--log-filename", default=Settings.LOG_FILENAME, help="Log file name"
    )
    parser.add_argument(
        "-r",
        "--max-retries",
        type=int,
        default=Settings.MAX_RETRIES,
        help="Maximum number of retries",
    )
    parser.add_argument(
        "-d",
        "--retry-delay",
        type=int,
        default=Settings.RETRY_DELAY,
        help="Retry delay in seconds",
    )
    parser.add_argument(
        "-v",
        "--verbosity",
        type=int,
        choices=[1, 2, 3],
        default=2,
        help="Verbosity level (1=error, 2=info, 3=debug)",
    )
    args = parser.parse_args()

    # Update settings based on command-line arguments
    Settings.OUTPUT_FOLDER = args.output_folder
    Settings.CSV_FILENAME = args.csv_filename
    Settings.LOG_FILENAME = args.log_filename
    Settings.MAX_RETRIES = args.max_retries
    Settings.RETRY_DELAY = args.retry_delay
    Settings.LOG_LEVEL = [logging.ERROR, logging.INFO, logging.DEBUG][
        args.verbosity - 1
    ]

    # Extract products. A store that never answered is an error, not an empty
    # catalogue, so it has to reach the exit code rather than a log line.
    try:
        extract_products(args.url, args.collections)
    except ShopifyRequestError as exc:
        # A store that did not answer needs the reason, not a traceback.
        logger.error("%s", exc)  # noqa: TRY400
        sys.exit(1)
