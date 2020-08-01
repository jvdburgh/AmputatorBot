import sys
import unittest
from datetime import datetime

from datahandlers.remote_datahandler import get_data, get_engine_session, add_data
from helpers import logger
from helpers.utils import get_urls, get_urls_info
from models.entry import Entry
from models.type import Type

log = logger.get_log(sys)


class BatchTest(unittest.TestCase):
    type = Type.TEST

    # Test the canonical-finding process with one example or specified data from the database
    def test_canonical(self, use_database=True):
        amount_of_canonicals = 0
        old_amount_of_canonicals = 0

        # Use data from the database
        if use_database:
            old_entries = get_data(
                session=get_engine_session(),
                limit=500,
                offset=5000,
                order_descending=True,
                canonical_url=False)

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

            urls = get_urls(old_entry.original_url)
            urls_info = get_urls_info(urls)
            if urls_info:
                for link in urls_info:
                    log.info(link.canonical_alt)

                    if link.amp_canonical:
                        log.info(link.amp_canonical)
                    if link.canonical:
                        amount_of_canonicals += 1

                    log.info(f"BODY   : {old_entry.original_url}")
                    log.info(f"OLD    : {old_entry.canonical_url}")
                    log.info(f"NEW    : {link.canonical}")

                    if link.canonical == old_entry.canonical_url:
                        log.info("It's the same!")
                    else:
                        log.info("It's not the same!")

                    """if link.canonical:
                        similarity = get_article_similarity(old_entry.original_url, link.canonical, log_articles=False)
                        log.info(f"Article similarity= {similarity}")"""

            else:
                log.warning(f"No canonicals found")

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

    # Test the send-to-database process
    def test_send_to_database(self):
        type = Type.TEST
        add_data(session=get_engine_session(),
                 entry_type=type.value,
                 handled_utc=(datetime.now().strftime('%Y-%m-%d %H:%M:%S')),
                 original_url="https://google.com/thisisatest",
                 canonical_url=None,
                 note="Test")

        self.assertTrue(True)
