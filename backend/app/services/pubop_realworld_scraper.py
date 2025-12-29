"""
pubop Real-World Data Scraper
Scrapes real news and posts about entities during simulation preparation

This service is called during the simulation preparation phase to:
1. For each entity (person, organization, topic), search for real news/posts
2. Use GenericWebCrawler to scrape news from multiple sources
3. Return RealDataSeed containing initial posts and trending topics
"""

import asyncio
from typing import List, Optional, Dict, Any
from datetime import datetime

from ..models.pubop import ScrapedPost, CrawlResult
from ..utils.logger import get_logger
from .crawler import LightPandaClient, GenericWebCrawler
from .pubop_bridge import PubopBridge, RealDataSeed

logger = get_logger('pubop.realworld_scraper')


# News sources to scrape for different regions/topics
NEWS_SOURCES = {
    "general": [
        "https://news.google.com",
        "https://www.reuters.com",
    ],
    "sea": [  # Southeast Asia
        "https://www.bangkokpost.com",
        "https://www.straitstimes.com",
        "https://www.channelnewsasia.com",
    ],
    "cambodia": [
        "https://www.phnompenhpost.com",
        "https://www.khmertimeskh.com",
    ],
    "thailand": [
        "https://www.bangkokpost.com",
        "https://www.nationthailand.com",
    ],
    "vietnam": [
        "https://e.vnexpress.net",
        "https://vietnamnews.vn",
    ],
}


class RealWorldScraper:
    """
    Scrapes real-world news and social media about simulation entities.
    
    Called during simulation preparation to inject real content
    into the simulation as initial posts and trending topics.
    
    Example:
        scraper = RealWorldScraper()
        seed = await scraper.scrape_entities(
            entities=["Bangkok Post", "Hun Manet", "Srettha Thavisin"],
            region="sea",
            max_posts_per_entity=5
        )
        # seed.initial_posts contains real scraped news
    """
    
    def __init__(
        self,
        engine: str = "browserless",
        max_posts_per_entity: int = 5,
        timeout: int = 30000
    ):
        """
        Initialize scraper.
        
        Args:
            engine: Browser engine ("browserless" or "lightpanda")
            max_posts_per_entity: Max articles per entity
            timeout: Browser timeout in ms
        """
        self.engine = engine
        self.max_posts_per_entity = max_posts_per_entity
        self.timeout = timeout
    
    async def scrape_entities(
        self,
        entities: List[Dict[str, Any]],
        region: str = "sea",
        max_total_posts: int = 50,
        progress_callback: Optional[callable] = None
    ) -> RealDataSeed:
        """
        Scrape real news for a list of entities.
        
        Args:
            entities: List of entity dicts with 'name' and 'type' keys
            region: Region for news sources (sea, cambodia, thailand, vietnam, general)
            max_total_posts: Max total posts to collect
            progress_callback: (current, total, message) callback
            
        Returns:
            RealDataSeed with scraped articles as initial_posts
        """
        all_posts: List[ScrapedPost] = []
        trending_topics: List[str] = []
        
        # Get news sources for region
        sources = NEWS_SOURCES.get(region, []) + NEWS_SOURCES.get("general", [])
        
        entity_names = [e.get("name", str(e)) if isinstance(e, dict) else str(e) for e in entities]
        total_entities = len(entity_names)
        
        logger.info(f"Starting real-world scrape for {total_entities} entities, region={region}")
        
        try:
            async with LightPandaClient(engine=self.engine, timeout=self.timeout) as client:
                crawler = GenericWebCrawler(client)
                
                for i, entity_name in enumerate(entity_names):
                    if len(all_posts) >= max_total_posts:
                        break
                    
                    if progress_callback:
                        progress_callback(
                            i + 1, total_entities,
                            f"Scraping news about: {entity_name}"
                        )
                    
                    # Search for entity in news
                    posts = await self._scrape_entity_news(
                        crawler, entity_name, sources,
                        limit=self.max_posts_per_entity
                    )
                    
                    if posts:
                        all_posts.extend(posts)
                        logger.info(f"Found {len(posts)} articles about {entity_name}")
                        
                        # Extract topics from posts
                        for post in posts:
                            trending_topics.extend(post.hashtags)
                    else:
                        logger.warning(f"No articles found for {entity_name}")
                    
                    # Small delay between entities
                    await asyncio.sleep(1)
        
        except Exception as e:
            logger.error(f"Error during real-world scraping: {e}")
            # Return partial results if we have any
        
        # Deduplicate trending topics
        trending_topics = list(set(trending_topics))[:20]
        
        # Convert posts to initial posts format
        bridge = PubopBridge(anonymize=False)
        initial_posts = []
        
        for post in all_posts:
            initial_posts.append({
                "user_id": 0,  # Will be assigned to an agent
                "username": post.author_name or "news_source",
                "content": post.content[:1500],  # Truncate
                "created_at": post.timestamp.isoformat() if post.timestamp else datetime.now().isoformat(),
                "likes": post.likes,
                "shares": post.shares,
                "comments": post.comments,
                "platform": post.platform,
                "source_url": post.url,
                "hashtags": post.hashtags,
                "is_real_world": True,  # Mark as real scraped data
            })
        
        logger.info(
            f"Real-world scraping complete: {len(all_posts)} articles, "
            f"{len(trending_topics)} topics"
        )
        
        return RealDataSeed(
            platform="web",
            query=f"entities: {', '.join(entity_names[:5])}...",
            crawled_at=datetime.now(),
            profiles=[],  # We don't generate profiles from news
            initial_posts=initial_posts,
            trending_topics=trending_topics,
            original_post_count=len(all_posts),
            original_user_count=0,
        )
    
    async def _scrape_entity_news(
        self,
        crawler: GenericWebCrawler,
        entity_name: str,
        sources: List[str],
        limit: int = 5
    ) -> List[ScrapedPost]:
        """Scrape news about a specific entity"""
        posts = []
        
        # Try Google News search first (best coverage)
        try:
            search_url = f"https://news.google.com/search?q={entity_name.replace(' ', '+')}"
            search_posts = await crawler.scrape_posts(search_url, limit=limit)
            posts.extend(search_posts)
        except Exception as e:
            logger.debug(f"Google News search failed for {entity_name}: {e}")
        
        # If not enough results, try specific sources
        if len(posts) < limit:
            for source in sources[:2]:  # Limit to 2 sources
                try:
                    search_posts = await crawler.scrape_posts(
                        f"{source}/search?q={entity_name.replace(' ', '+')}",
                        limit=limit - len(posts)
                    )
                    posts.extend(search_posts)
                    
                    if len(posts) >= limit:
                        break
                except Exception as e:
                    logger.debug(f"Source {source} search failed: {e}")
        
        return posts[:limit]
    
    async def scrape_entity_names(
        self,
        entity_names: List[str],
        region: str = "sea",
        max_total_posts: int = 50,
        progress_callback: Optional[callable] = None
    ) -> RealDataSeed:
        """
        Convenience method to scrape by entity names directly.
        
        Args:
            entity_names: List of entity name strings
            region: Region for news sources
            max_total_posts: Max total posts
            progress_callback: Progress callback
            
        Returns:
            RealDataSeed
        """
        entities = [{"name": name} for name in entity_names]
        return await self.scrape_entities(
            entities, region, max_total_posts, progress_callback
        )


def sync_scrape_entities(
    entity_names: List[str],
    region: str = "sea",
    max_posts: int = 50
) -> RealDataSeed:
    """
    Synchronous wrapper for scraping entities.
    
    Useful for calling from synchronous code.
    
    Args:
        entity_names: List of entity name strings
        region: Region for news sources
        max_posts: Max total posts
        
    Returns:
        RealDataSeed
    """
    scraper = RealWorldScraper()
    loop = asyncio.new_event_loop()
    try:
        return loop.run_until_complete(
            scraper.scrape_entity_names(entity_names, region, max_posts)
        )
    finally:
        loop.close()
