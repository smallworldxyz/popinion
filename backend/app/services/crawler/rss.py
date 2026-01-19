"""
RSS Crawler
Fetches and parses RSS feeds for news ingestion.
"""

import asyncio
import feedparser
from typing import List, Optional
from datetime import datetime
from email.utils import parsedate_to_datetime

from ...models.pubop import ScrapedPost, ScrapedUser, ScrapedTrend, Platform
from ...utils.logger import get_logger
from .base import BaseCrawler
from .client import LightPandaClient

logger = get_logger('pubop.crawler.rss')

class RSSCrawler(BaseCrawler):
    """
    Crawler for RSS Feeds.
    
    Can parse standard RSS/Atom feeds.
    Optionally traverses links to scrape full content.
    """
    
    PLATFORM = "rss"
    
    async def scrape_posts(
        self, 
        query: str, 
        limit: int = 50
    ) -> List[ScrapedPost]:
        """
        Fetch posts from an RSS feed URL.
        
        Args:
            query: The RSS Feed URL
            limit: Max items to process
            
        Returns:
            List of ScrapedPost objects
        """
        feed_url = query
        return await self.scrape_feed(feed_url, limit)

    async def scrape_feed(
        self,
        feed_url: str,
        limit: int = 50
    ) -> List[ScrapedPost]:
        """
        Parse RSS feed and convert to ScrapedPost objects.
        """
        logger.info(f"Fetching RSS feed: {feed_url}")
        posts = []
        
        try:
            # feedparser is synchronous, run in executor
            feed = await asyncio.to_thread(feedparser.parse, feed_url)
            
            if feed.bozo:
                logger.warning(f"RSS Feed parsing warning for {feed_url}: {feed.bozo_exception}")
            
            feed_title = feed.feed.get('title', 'Unknown Source')
            
            for entry in feed.entries[:limit]:
                try:
                    # Parse timestamp
                    timestamp = datetime.now()
                    if hasattr(entry, 'published_parsed') and entry.published_parsed:
                        timestamp = datetime(*entry.published_parsed[:6])
                    elif hasattr(entry, 'updated_parsed') and entry.updated_parsed:
                        timestamp = datetime(*entry.updated_parsed[:6])
                        
                    content = entry.get('summary', '') or entry.get('description', '')
                    # If content is HTML, might want to strip tags or keep as is? 
                    # For now keep raw, downstream can clean.
                    
                    post_id = entry.get('id', entry.get('link'))
                    
                    posts.append(ScrapedPost(
                        platform="rss",
                        post_id=str(post_id)[:100], # ensure not too long
                        content=f"{entry.title}\n\n{content}",
                        author_id=feed_title,
                        author_name=feed_title, # Source name as author
                        timestamp=timestamp,
                        url=entry.get('link'),
                        views=0,
                        media_urls=[], # RSS rarely has easy media urls without parsing HTML
                        is_news=True   # Flag as news
                    ))
                except Exception as e:
                    logger.warning(f"Error parsing RSS entry: {e}")
                    continue
                    
            logger.info(f"Parsed {len(posts)} items from {feed_title}")
            return posts

        except Exception as e:
            logger.error(f"RSS scrape failed for {feed_url}: {e}")
            return []

    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        return None # Not applicable

    async def scrape_trending(self) -> List[ScrapedTrend]:
        return [] # Not applicable
