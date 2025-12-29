"""
Zalo Crawler
Scrapes Zalo News articles (Vietnam-focused)
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

logger = get_logger('pubop.crawler.zalo')


class ZaloCrawler(BaseCrawler):
    """
    Crawler for Zalo News (Vietnam).
    
    Zalo is Vietnam's dominant messaging platform with integrated news.
    This crawler focuses on public Zalo News content.
    
    Note: Zalo's main functionality requires Vietnamese phone verification.
    This crawler only accesses public news content.
    
    Example:
        async with LightPandaClient(engine="browserless") as client:
            crawler = ZaloCrawler(client)
            posts = await crawler.scrape_posts(limit=20)
    """
    
    PLATFORM = "zalo"
    BASE_URL = "https://news.zalo.me"
    
    # Selectors for Zalo News
    SELECTORS = {
        "article": '[class*="article"], [class*="card"], [class*="item"]',
        "article_link": 'a[href*="/article/"], a[href*="/news/"]',
        "article_title": 'h1, h2, h3, [class*="title"]',
        "category": '[class*="category"], [class*="topic"]',
    }
    
    async def scrape_posts(
        self, 
        query: str = None, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape Zalo News articles.
        
        Args:
            query: Optional search query or category
            limit: Maximum articles to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        if query:
            url = f"{self.BASE_URL}/tim-kiem?q={quote_plus(query)}"
        else:
            url = self.BASE_URL
        
        logger.info(f"Scraping Zalo News: {query or 'homepage'}")
        return await self._scrape_articles_from_url(url, query or "zalo_news", limit)
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape articles from a Zalo News category.
        
        Args:
            channel_id: Category name (e.g., "thoi-su", "the-gioi", "kinh-doanh")
            limit: Maximum articles to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        category = channel_id.lower().strip()
        url = f"{self.BASE_URL}/{category}"
        
        logger.info(f"Scraping Zalo News category: {category}")
        return await self._scrape_articles_from_url(url, category, limit)
    
    async def _scrape_articles_from_url(
        self, 
        url: str, 
        source_id: str, 
        limit: int
    ) -> List[ScrapedPost]:
        """Internal method to scrape articles from Zalo News"""
        posts = []
        
        try:
            page = await self._get_page()
            
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(3)
            
            # Find article links
            article_links = []
            for selector in ['a[href*="/article/"]', 'a[href*=".html"]', '[class*="card"] a', 'article a']:
                elements = await page.query_selector_all(selector)
                for el in elements:
                    href = await el.get_attribute("href")
                    if href and href not in article_links:
                        if not href.startswith("http"):
                            href = f"{self.BASE_URL}{href}"
                        article_links.append(href)
                if len(article_links) >= limit:
                    break
            
            if not article_links:
                logger.warning(f"No articles found at {url}")
                return posts
            
            logger.info(f"Found {len(article_links)} article links")
            
            # Scroll to load more
            scroll_count = 0
            while len(article_links) < limit and scroll_count < 10:
                await page.evaluate("window.scrollBy(0, 1000)")
                await asyncio.sleep(2)
                
                for selector in ['a[href*="/article/"]', 'a[href*=".html"]']:
                    elements = await page.query_selector_all(selector)
                    for el in elements:
                        href = await el.get_attribute("href")
                        if href and href not in article_links:
                            if not href.startswith("http"):
                                href = f"{self.BASE_URL}{href}"
                            article_links.append(href)
                
                scroll_count += 1
            
            # Parse articles (basic info)
            for article_url in article_links[:limit]:
                try:
                    # Extract article ID from URL
                    article_id_match = re.search(r'/([^/]+)\.html', article_url)
                    if not article_id_match:
                        article_id_match = re.search(r'/article/([^/?]+)', article_url)
                    article_id = article_id_match.group(1) if article_id_match else str(hash(article_url))[:12]
                    
                    post = ScrapedPost(
                        platform=self.PLATFORM,
                        post_id=article_id,
                        content=f"Zalo News article: {article_id}",  # Would need to visit page for full content
                        author_id="Zalo News",
                        author_name="Zalo News",
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
                    logger.warning(f"Error parsing Zalo article: {e}")
            
        except Exception as e:
            logger.error(f"Error scraping Zalo News: {e}")
            raise
        
        return posts
    
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        Zalo user profiles require authentication.
        Returns None.
        """
        logger.info("Zalo user profiles require Vietnamese phone authentication")
        return None
    
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """
        Scrape Zalo News trending/hot topics.
        """
        trends = []
        url = f"{self.BASE_URL}/tin-nong"  # Hot news section
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(2)
            
            # Look for trending/hot articles
            hot_elements = await page.query_selector_all('[class*="hot"] a, [class*="trending"] a, [class*="top"] a')
            
            for i, element in enumerate(hot_elements[:10]):
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
            logger.error(f"Error scraping Zalo trending: {e}")
        
        return trends
