import sys
import unittest
from datetime import datetime

from datahandlers.remote_datahandler import get_data, get_engine_session, add_data, get_entry_by_original_url
from helpers import logger
from helpers.utils import get_urls, get_urls_info
from models.entry import Entry
from models.link import CanonicalType
from models.type import Type

log = logger.get_log(sys)


class BatchTest(unittest.TestCase):
    type = Type.TEST

    # Test the canonical-finding process with one example or specified data from the database
    def test_canonical(self, use_database=True):
        new_canonicals_amount = 0
        old_canonicals_amount = 0

        # Use data from the database
        if use_database:
            old_entries = get_data(
                session=get_engine_session(),
                limit=100,
                offset=5000,
                order_descending=True,
                canonical_url=None
            )

        # Or use a single entry as specified below
        else:
            old_entries = [Entry(
                original_url="www.google.com/amp/s/abc3340.com/amp/news/inside-your-world/the-federal-government-spends-billions-each-year-maintaining-empty-buildings-nationwide",
                canonical_url="https://abc3340.com/news/inside-your-world/the-federal-government-spends-billions-each-year-maintaining-empty-buildings-nationwide"
            )]

        # Loop through every old entry and try to find the canonicals, compare the results
        for old_entry in old_entries:
            if old_entry.canonical_url:
                old_canonicals_amount += 1

            urls = get_urls(old_entry.original_url)
            urls_info = get_urls_info(urls)
            if urls_info:
                for link in urls_info:
                    if link.canonical:
                        new_canonicals_amount += 1
                        if link.canonical.url == old_entry.canonical_url:
                            log.info("Canonical URLs match")
                        else:
                            log.warning("Canonical URLs do not match!")

                    log.info(f"BODY : {old_entry.original_url}")
                    log.info(f"OLD  : {old_entry.canonical_url}")
                    log.info(f"NEW  : {link.canonical.url if link.canonical else None}")

            else:
                log.warning(f"No canonicals found")

        log.info(f"\nCanonicals found: Old: {old_canonicals_amount}, New: {new_canonicals_amount}")

        # If same as before, great!
        if new_canonicals_amount == old_canonicals_amount:
            self.assertEqual(new_canonicals_amount, old_canonicals_amount)

        # If better than before, great!
        if new_canonicals_amount > old_canonicals_amount:
            self.assertGreater(new_canonicals_amount, old_canonicals_amount)

        # If it is worse than before, not good.
        if new_canonicals_amount < old_canonicals_amount:
            self.assertLess(old_canonicals_amount, new_canonicals_amount)

    # Test the send-to-database process
    def test_send_to_database(self):
        type = Type.TEST
        add_data(session=get_engine_session(),
                 entry_type=type.value,
                 handled_utc=(datetime.now().strftime('%Y-%m-%d %H:%M:%S')),
                 original_url="https://google.com/thisisatest",
                 canonical_url=None,
                 canonical_type=CanonicalType.DATABASE,
                 note="Test")

        self.assertTrue(True)

    def test_get_canonical_from_database_by_url(self, use_database=False):
        amount_of_correct_retrievals = 0
        session = get_engine_session()

        # Use data from the database
        if use_database:
            old_entries = get_data(
                session=session,
                limit=100,
                offset=5000,
                order_descending=True,
                canonical_url=True)

        # Or use a single entry as specified below
        else:
            old_entries = [Entry(
                original_url="https://www.mynbc5.com/amp/article/emily-ferlazzo-joseph-bolton-vermont-missing-update/38004866",
                canonical_url="https://abc3340.com/news/inside-your-world/the-federal-government-spends-billions-each-year-maintaining-empty-buildings-nationwide"
            )]

        for old_entry in old_entries:
            log.info("OLD")
            log.info(old_entry.entry_id)
            log.info(old_entry.canonical_url)
            found_entry = get_entry_by_original_url(old_entry.original_url, session)

            if found_entry:
                log.info("NEW")
                log.info(found_entry.entry_id)
                log.info(old_entry.canonical_url)

                if old_entry.entry_id == found_entry.entry_id:
                    amount_of_correct_retrievals += 1

            else:
                log.warning("No entry found!")

        self.assertEqual(amount_of_correct_retrievals, old_entries.len)
