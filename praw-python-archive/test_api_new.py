import sys
import unittest

import requests

from datahandlers.remote_datahandler import get_data, get_engine_session
from helpers import logger
from models.entry import Entry
from models.type import Type

log = logger.get_log(sys)


class BatchTest(unittest.TestCase):

    # Test the canonical-finding process with one example or specified data from the database
    def test_canonical(self, use_database=True):
        amount_of_canonicals = 0
        old_amount_of_canonicals = 0

        # Use data from the database
        if use_database:
            old_entries = get_data(
                session=get_engine_session(),
                limit=100,
                offset=5000,
                order_descending=False,
                canonical_url=True)

        # Or use a single entry as specified below
        else:
            old_entries = [Entry(
                original_url="https://www.google.com/amp/s/abc3340.com/amp/news/inside-your-world/the-federal-government-spends-billions-each-year-maintaining-empty-buildings-nationwide",
                canonical_url="https://abc3340.com/news/inside-your-world/the-federal-government-spends-billions-each-year-maintaining-empty-buildings-nationwide"
            )]

        # Loop through every old entry and try to find the canonicals, compare the results
        for old_entry in old_entries:
            if old_entry.canonical_url:
                old_amount_of_canonicals += 1

            base_api_url = "https://www.amputatorbot.com/api/v1/convert?q="
            api_url_append1 = old_entry.original_url
            url = base_api_url + api_url_append1

            result = requests.get(url)
            log.info(result.status_code)

            if result.status_code == 200:
                amount_of_canonicals += 1
            else:
                log.info(f"Old: {old_entry.canonical_url}, New: {result.text}")
                log.info(old_entry.original_url)
                log.info(old_entry.canonical_url)

        log.info(f"\nCanonicals found: Old: {old_amount_of_canonicals}, New: {amount_of_canonicals}")

        # If same as before, great!
        if amount_of_canonicals == old_amount_of_canonicals:
            self.assertEqual(amount_of_canonicals, old_amount_of_canonicals)
        # If it is better than before, great!
        if amount_of_canonicals > old_amount_of_canonicals:
            self.assertGreater(amount_of_canonicals, old_amount_of_canonicals)
        # If it is worse than before, not good.
        if amount_of_canonicals < old_amount_of_canonicals:
            self.assertLess(old_amount_of_canonicals, amount_of_canonicals)

