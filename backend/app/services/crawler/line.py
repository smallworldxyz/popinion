"""
LINE Crawler
Scrapes LINE Today news and articles (Thailand-focused)
"""

import re
import asyncio
from typing import List, Optional
from datetime import datetime
from urllib.parse import quote_plus

from ...models.pubop import ScrapedPost, ScrapedUser, ScrapedTrend, Platform
from ...utils.logger import get_logger
from .base import BaseCrawler
from .client import LightPandaClient

logger = get_logger('pubop.crawler.line')


class LINECrawler(BaseCrawler):
    """
    Crawler for LINE Today news (Thailand).
    
    LINE Today is a news aggregation platform integrated with LINE messenger,
    very popular in Thailand. This crawler focuses on public news content.
    
    Note: LINE's main chat functionality requires authentication.
    This crawler only accesses public LINE Today content.
    
    Example:
        async with LightPandaClient(engine="browserless") as client:
            crawler = LINECrawler(client)
            posts = await crawler.scrape_posts("Thailand", limit=20)
    """
    
    PLATFORM = "line"
    BASE_URL = "https://today.line.me"
    COUNTRY = "th"  # Thailand
    
    # Selectors for LINE Today
    SELECTORS = {
        "article": 'article, [class*="articleCard"], [class*="card"]',
        "article_link": 'a[href*="/article/"]',
        "article_title": 'h1, h2, h3, [class*="title"]',
        "article_source": '[class*="source"], [class*="publisher"]',
        "trending": '[class*="trending"], [class*="popular"]',
    }
    
    async def scrape_posts(
        self, 
        query: str = None, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape LINE Today news articles.
        
        Args:
            query: Optional topic/category to search
            limit: Maximum articles to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        if query:
            url = f"{self.BASE_URL}/{self.COUNTRY}/v2/search/article?q={quote_plus(query)}"
        else:
            url = f"{self.BASE_URL}/{self.COUNTRY}"
        
        logger.info(f"Scraping LINE Today: {query or 'homepage'}")
        return await self._scrape_articles_from_url(url, query or "line_today", limit)
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape articles from a LINE Today category/channel.
        
        Args:
            channel_id: Category name (e.g., "news", "entertainment", "sports")
            limit: Maximum articles to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        # Map common category names
        category = channel_id.lower().strip()
        url = f"{self.BASE_URL}/{self.COUNTRY}/v2/tab/{category}"
        
        logger.info(f"Scraping LINE Today category: {category}")
        return await self._scrape_articles_from_url(url, category, limit)
    
    async def _scrape_articles_from_url(
        self, 
        url: str, 
        source_id: str, 
        limit: int
    ) -> List[ScrapedPost]:
        """Internal method to scrape articles from LINE Today"""
        posts = []
        
        try:
            page = await self._get_page()
            
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(3)
            
            # Find article elements
            article_elements = []
            for selector in ['article', '[class*="articleCard"]', '[class*="card"] a[href*="/article/"]', 'a[href*="/article/"]']:
                article_elements = await page.query_selector_all(selector)
                if article_elements:
                    break
            
            if not article_elements:
                logger.warning(f"No articles found at {url}")
                return posts
            
            logger.info(f"Found {len(article_elements)} article elements")
            
            # Scroll to load more
            scroll_count = 0
            while len(article_elements) < limit and scroll_count < 10:
                await page.evaluate("window.scrollBy(0, 1000)")
                await asyncio.sleep(2)
                
                for selector in ['article', 'a[href*="/article/"]']:
                    article_elements = await page.query_selector_all(selector)
                    if len(article_elements) >= limit:
                        break
                
                scroll_count += 1
            
            # Extract article info
            article_urls = []
            for element in article_elements[:limit * 2]:  # Get extra for dedup
                # Get article URL
                href = None
                
                # Element might be the link itself or contain a link
                if await element.get_attribute("href"):
                    href = await element.get_attribute("href")
                else:
                    link_el = await element.query_selector('a[href*="/article/"]')
                    if link_el:
                        href = await link_el.get_attribute("href")
                
                if href and "/article/" in href:
                    if not href.startswith("http"):
                        href = f"{self.BASE_URL}{href}"
                    if href not in article_urls:
                        article_urls.append(href)
                        if len(article_urls) >= limit:
                            break
            
            logger.info(f"Found {len(article_urls)} unique article URLs")
            
            # Parse articles (basic info from list view)
            for article_url in article_urls[:limit]:
                try:
                    # Extract article ID from URL
                    article_id_match = re.search(r'/article/([^?/]+)', article_url)
                    article_id = article_id_match.group(1) if article_id_match else str(hash(article_url))[:12]
                    
                    post = ScrapedPost(
                        platform=self.PLATFORM,
                        post_id=article_id,
                        content=f"LINE Today article: {article_id}",  # Would need to visit page for full content
                        author_id="LINE Today",
                        author_name="LINE Today",
                        timestamp=datetime.now(),
                        url=article_url,
                        likes=0,
                        shares=0,
                        comments=0,
                        media_urls=[],
                        hashtags=[],
                    )
                    posts.append(post)
                    
                except Exception as e:
                    logger.warning(f"Error parsing LINE article: {e}")
            
        except Exception as e:
            logger.error(f"Error scraping LINE Today: {e}")
            raise
        
        return posts
    
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        LINE doesn't have public user profiles accessible via web.
        Returns None.
        """
        logger.info("LINE user profiles not accessible via web scraping")
        return None
    
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """
        Scrape LINE Today trending articles.
        """
        trends = []
        url = f"{self.BASE_URL}/{self.COUNTRY}"
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(2)
            
            # Look for trending/popular section
            trending_elements = await page.query_selector_all('[class*="trending"] a, [class*="popular"] a, [class*="top"] a')
            
            for i, element in enumerate(trending_elements[:10]):
                href = await element.get_attribute("href")
                text = await element.inner_text()
                
                if href and text:
                    trends.append(ScrapedTrend(
                        platform=self.PLATFORM,
                        name=text.strip()[:100],
                        url=href if href.startswith("http") else f"{self.BASE_URL}{href}",
                        rank=i + 1,
                    ))
            
        except Exception as e:
            logger.error(f"Error scraping LINE trending: {e}")
        
        return trends
