import sys
import traceback
from difflib import SequenceMatcher
from random import choice
from typing import Optional

from newspaper import ArticleException, Article

from helpers import logger
from static import static

log = logger.get_log(sys)


# Check how similar the article text is
def get_article_similarity(url1, url2, log_articles=False) -> Optional[float]:
    try:
        # Download and parse first article
        article1 = Article(url1, browser_user_agent=choice(static.HEADERS))
        article1.download()
        article1.parse()
        article1_text = article1.text

        # Download and parse second article
        article2 = Article(url2, browser_user_agent=choice(static.HEADERS))
        article2.download()
        article2.parse()
        article2_text = article2.text

        if log_articles:
            log.debug(f"Article 1: {article1_text}\n\nArticle 2: {article2_text}")

        # Compare the two articles and return the ratio (0-1)
        return SequenceMatcher(None, article1_text, article2_text).ratio()

    except (ArticleException, Exception):
        log.error(traceback.format_exc())
        log.warning("Couldn't compare articles")
        return None
