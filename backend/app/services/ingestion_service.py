"""
Ingestion Service
Orchestrates multi-platform sentiment ingestion for SEA via lightpanda-crawler.
"""

from typing import List, Dict, Any, Optional
from datetime import datetime
import asyncio

from ..models.pubop import ScrapedPost
from .crawler.client import LightPandaClient
from .crawler.rss import RSSCrawler
from .crawler.telegram import TelegramCrawler
from .crawler.base import BaseCrawler
from ..utils.logger import get_logger

logger = get_logger('pubop.services.ingestion')

class IngestionService:
    """
    Unified Ingestion Service.
    
    Supports:
    - RSS Feeds (Rappler, CNA, etc.) -> Low Friction
    - Telegram Public Channels -> Medium Friction
    """
    
    # Default SEA News Sources
    DEFAULT_RSS_SOURCES = [
        "https://www.rappler.com/feed",
        "https://www.channelnewsasia.com/api/v1/rss-feeds/rss_feed_1001", # CNA Top Stories
        "https://www.thejakartapost.com/rss/latest-news",
        "https://www.bangkokpost.com/rss/data/topstories.xml"
    ]
    
    async def ingest_live_sentiment(
        self,
        sources: Optional[List[str]] = None,
        keywords: Optional[List[str]] = None,
        limit_per_source: int = 20
    ) -> Dict[str, Any]:
        """
        Trigger live ingestion job.
        
        Args:
            sources: List of sources (URL or channel handle)
            keywords: Filter keywords (client-side filter for RSS)
            limit_per_source: Max items per source
            
        Returns:
            Ingestion stats
        """
        sources = sources or self.DEFAULT_RSS_SOURCES
        all_posts: List[ScrapedPost] = []
        errors = []
        
        logger.info(f"Starting ingestion for {len(sources)} sources")
        
        # Split sources by type
        rss_urls = [s for s in sources if s.startswith('http')]
        telegram_channels = [s for s in sources if not s.startswith('http') and not s.startswith('@')] 
        # Assume non-http strings are telegram channels for now (or twitter handles, but we only have TG crawler)
        # Better: Client should specify type, or we heuristically detect
        
        async with LightPandaClient(engine="browserless") as client:
            # 1. Process RSS (Parallel)
            if rss_urls:
                rss_crawler = RSSCrawler(client)
                rss_tasks = [rss_crawler.scrape_feed(url, limit_per_source) for url in rss_urls]
                rss_results = await asyncio.gather(*rss_tasks, return_exceptions=True)
                
                for i, res in enumerate(rss_results):
                    if isinstance(res, Exception):
                        logger.error(f"RSS failed for {rss_urls[i]}: {res}")
                        errors.append(str(res))
                    else:
                        all_posts.extend(res)

            # 2. Process Telegram (Sequential to avoid rate limits/blocks)
            if telegram_channels:
                tg_crawler = TelegramCrawler(client)
                for channel in telegram_channels:
                    try:
                        logger.info(f"Scraping Telegram: {channel}")
                        posts = await tg_crawler.scrape_channel(channel, limit=limit_per_source)
                        all_posts.extend(posts)
                        await asyncio.sleep(2) # Polite delay
                    except Exception as e:
                        logger.error(f"Telegram failed for {channel}: {e}")
                        errors.append(str(e))
        
        # 3. Filter by keywords if provided
        if keywords:
            keywords_lower = [k.lower() for k in keywords]
            filtered_posts = []
            for post in all_posts:
                content_lower = (post.content or "").lower()
                if any(k in content_lower for k in keywords_lower):
                    filtered_posts.append(post)
            
            logger.info(f"Keyword filtered: {len(all_posts)} -> {len(filtered_posts)}")
            all_posts = filtered_posts
            
        # 4. Save to DB (Neo4j or Temp Storage)
        # For now, just return results. 
        # TODO: Persist to Neo4j as 'RealPost' nodes
        
        return {
            "success": True,
            "total_ingested": len(all_posts),
            "source_count": len(sources),
            "errors": errors,
            "sample_data": [p.dict() for p in all_posts[:5]]
        }

    # Singleton helper
    @classmethod
    async def run_now(cls, **kwargs):
        service = cls()
        return await service.ingest_live_sentiment(**kwargs)
