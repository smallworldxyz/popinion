"""
Base Crawler Interface
Abstract base class for platform-specific crawlers
"""

from abc import ABC, abstractmethod
from typing import List, Optional, Any
from datetime import datetime

from ...models.pubop import ScrapedPost, ScrapedUser, ScrapedTrend, CrawlResult
from ...utils.logger import get_logger
from .client import LightPandaClient

logger = get_logger('pubop.crawler.base')


class BaseCrawler(ABC):
    """
    Abstract base class for platform-specific web crawlers.
    
    All platform crawlers (Telegram, Twitter, Facebook, etc.) should
    inherit from this class and implement the abstract methods.
    
    Usage:
        class TelegramCrawler(BaseCrawler):
            async def scrape_posts(self, query, limit=100):
                # Platform-specific implementation
                ...
    """
    
    # Platform identifier (override in subclass)
    PLATFORM: str = "unknown"
    
    def __init__(self, client: LightPandaClient):
        """
        Initialize crawler with LightPanda client.
        
        Args:
            client: Connected LightPandaClient instance
        """
        self.client = client
        self._page = None
    
    async def _get_page(self) -> Any:
        """Get or create a browser page"""
        if self._page is None:
            self._page = await self.client.new_page()
        return self._page
    
    async def _close_page(self) -> None:
        """Close the browser page"""
        if self._page:
            try:
                await self._page.close()
            except Exception as e:
                logger.warning(f"Error closing page: {e}")
            self._page = None
    
    @abstractmethod
    async def scrape_posts(
        self, 
        query: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape posts matching a search query.
        
        Args:
            query: Search query or topic
            limit: Maximum number of posts to scrape
            
        Returns:
            List of ScrapedPost objects
        """
        pass
    
    @abstractmethod
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        Scrape a user profile.
        
        Args:
            user_id: User identifier (username or ID)
            
        Returns:
            ScrapedUser object or None if not found
        """
        pass
    
    @abstractmethod
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """
        Scrape trending topics.
        
        Returns:
            List of ScrapedTrend objects
        """
        pass
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape posts from a channel/page/group.
        
        Args:
            channel_id: Channel identifier
            limit: Maximum number of posts to scrape
            
        Returns:
            List of ScrapedPost objects
        """
        # Default implementation - override in subclass if supported
        logger.warning(f"scrape_channel not implemented for {self.PLATFORM}")
        return []
    
    async def crawl(
        self, 
        query: Optional[str] = None,
        channel_id: Optional[str] = None,
        include_users: bool = False,
        include_trends: bool = False,
        limit: int = 100
    ) -> CrawlResult:
        """
        Perform a complete crawl operation.
        
        Args:
            query: Search query (optional)
            channel_id: Channel to scrape (optional)
            include_users: Whether to scrape user profiles
            include_trends: Whether to scrape trending topics
            limit: Maximum posts to scrape
            
        Returns:
            CrawlResult with all scraped data
        """
        result = CrawlResult(
            platform=self.PLATFORM,
            query=query,
            crawled_at=datetime.now()
        )
        
        try:
            # Scrape posts
            if query:
                logger.info(f"Scraping posts for query: {query}")
                result.posts = await self.scrape_posts(query, limit=limit)
            elif channel_id:
                logger.info(f"Scraping channel: {channel_id}")
                result.posts = await self.scrape_channel(channel_id, limit=limit)
            
            # Scrape user profiles
            if include_users and result.posts:
                author_ids = list(set(p.author_id for p in result.posts))[:20]
                logger.info(f"Scraping {len(author_ids)} user profiles")
                for author_id in author_ids:
                    user = await self.scrape_user(author_id)
                    if user:
                        result.users.append(user)
            
            # Scrape trends
            if include_trends:
                logger.info("Scraping trending topics")
                result.trends = await self.scrape_trending()
            
            result.success = True
            logger.info(
                f"Crawl complete: {len(result.posts)} posts, "
                f"{len(result.users)} users, {len(result.trends)} trends"
            )
            
        except Exception as e:
            logger.error(f"Crawl failed: {e}")
            result.success = False
            result.error = str(e)
        
        finally:
            await self._close_page()
        
        return result
