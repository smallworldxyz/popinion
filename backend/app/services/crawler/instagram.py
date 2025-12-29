"""
Instagram Crawler
Scrapes public Instagram profiles and posts
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

logger = get_logger('pubop.crawler.instagram')


class InstagramCrawler(BaseCrawler):
    """
    Crawler for public Instagram profiles and posts.
    
    Scrapes content from instagram.com public profiles.
    
    Note: Instagram has strong anti-bot measures. This crawler
    works best with Browserless (not LightPanda) due to complex JS.
    
    Example:
        async with LightPandaClient(engine="browserless") as client:
            crawler = InstagramCrawler(client)
            posts = await crawler.scrape_channel("instagram", limit=20)
    """
    
    PLATFORM = Platform.TWITTER.value  # Using "twitter" as placeholder, will add INSTAGRAM to enum
    BASE_URL = "https://www.instagram.com"
    
    # Selectors for Instagram
    SELECTORS = {
        "post": 'article a[href*="/p/"], article a[href*="/reel/"]',
        "post_container": 'article',
        "post_image": 'article img[src*="cdninstagram"]',
        "profile_name": 'header section h2, header h1',
        "profile_bio": 'header section > div:last-child',
        "stats": 'header section ul li',
        "post_link": 'a[href*="/p/"], a[href*="/reel/"]',
    }
    
    async def scrape_posts(
        self, 
        query: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Search Instagram for posts (hashtag search).
        
        Args:
            query: Hashtag to search (without #)
            limit: Maximum posts to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        # Clean query - treat as hashtag
        hashtag = query.strip().lstrip('#')
        url = f"{self.BASE_URL}/explore/tags/{quote_plus(hashtag)}/"
        
        logger.info(f"Searching Instagram hashtag: #{hashtag}")
        return await self._scrape_posts_from_url(url, hashtag, limit)
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape posts from a public Instagram profile.
        
        Args:
            channel_id: Username (with or without @)
            limit: Maximum posts to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        username = channel_id.strip().lstrip('@')
        url = f"{self.BASE_URL}/{username}/"
        
        logger.info(f"Scraping Instagram profile: @{username}")
        return await self._scrape_posts_from_url(url, username, limit)
    
    async def _scrape_posts_from_url(
        self, 
        url: str, 
        source_id: str, 
        limit: int
    ) -> List[ScrapedPost]:
        """Internal method to scrape posts from any Instagram page"""
        posts = []
        
        try:
            page = await self._get_page()
            
            await page.goto(url, wait_until="networkidle")
            await asyncio.sleep(3)  # Instagram loads dynamically
            
            # Check for login wall
            login_wall = await page.query_selector('[href*="/accounts/login"]')
            if login_wall:
                logger.warning("Instagram login wall detected - limited access")
            
            # Find post links
            post_links = []
            for selector in ['a[href*="/p/"]', 'a[href*="/reel/"]']:
                elements = await page.query_selector_all(selector)
                for el in elements:
                    href = await el.get_attribute("href")
                    if href and href not in post_links:
                        post_links.append(href)
            
            if not post_links:
                logger.warning(f"No posts found at {url}")
                return posts
            
            # Scroll to load more posts
            scroll_count = 0
            while len(post_links) < limit and scroll_count < 10:
                await page.evaluate("window.scrollBy(0, 1000)")
                await asyncio.sleep(2)
                
                for selector in ['a[href*="/p/"]', 'a[href*="/reel/"]']:
                    elements = await page.query_selector_all(selector)
                    for el in elements:
                        href = await el.get_attribute("href")
                        if href and href not in post_links:
                            post_links.append(href)
                
                scroll_count += 1
            
            logger.info(f"Found {len(post_links)} post links")
            
            # Scrape individual posts (limit to avoid rate limiting)
            for post_url in post_links[:limit]:
                post = await self._scrape_single_post(page, post_url, source_id)
                if post:
                    posts.append(post)
                
                # Small delay between posts
                await asyncio.sleep(0.5)
            
        except Exception as e:
            logger.error(f"Error scraping Instagram: {e}")
            raise
        
        return posts
    
    async def _scrape_single_post(
        self, 
        page, 
        post_url: str, 
        source_id: str
    ) -> Optional[ScrapedPost]:
        """Scrape a single Instagram post"""
        try:
            # Make URL absolute if needed
            if not post_url.startswith("http"):
                post_url = f"{self.BASE_URL}{post_url}"
            
            await page.goto(post_url, wait_until="domcontentloaded")
            await asyncio.sleep(2)
            
            # Extract post ID from URL (/p/ABC123/ or /reel/ABC123/)
            post_id_match = re.search(r'/(?:p|reel)/([^/]+)', post_url)
            post_id = post_id_match.group(1) if post_id_match else str(hash(post_url))[:12]
            
            # Get caption/content
            content = ""
            for selector in ['article span', 'article h1', 'meta[property="og:description"]']:
                if selector.startswith('meta'):
                    el = await page.query_selector(selector)
                    if el:
                        content = await el.get_attribute("content") or ""
                else:
                    elements = await page.query_selector_all(selector)
                    for el in elements:
                        text = await el.inner_text()
                        if text and len(text) > len(content) and len(text) < 2000:
                            content = text
            
            # Get author
            author_name = source_id
            for selector in ['article header a', 'a[href*="/"][role="link"]']:
                author_el = await page.query_selector(selector)
                if author_el:
                    href = await author_el.get_attribute("href")
                    if href and "/p/" not in href and "/reel/" not in href:
                        name = href.strip("/").split("/")[-1]
                        if name and len(name) < 50:
                            author_name = name
                            break
            
            # Get likes count
            likes = 0
            for selector in ['section span', 'a[href*="liked_by"]']:
                likes_el = await page.query_selector(selector)
                if likes_el:
                    text = await likes_el.inner_text()
                    likes = self._parse_count(text)
                    if likes > 0:
                        break
            
            # Get comments count
            comments = 0
            comments_el = await page.query_selector('a[href*="comments"]')
            if comments_el:
                text = await comments_el.inner_text()
                comments = self._parse_count(text)
            
            # Get media URLs
            media_urls = []
            for selector in ['article img[src*="cdninstagram"]', 'article video source']:
                media_elements = await page.query_selector_all(selector)
                for el in media_elements[:5]:
                    src = await el.get_attribute("src")
                    if src and "s150x150" not in src:  # Skip thumbnails
                        media_urls.append(src)
            
            # Extract hashtags
            hashtags = re.findall(r'#(\w+)', content)
            
            return ScrapedPost(
                platform="instagram",
                post_id=post_id,
                content=content.strip()[:2000],
                author_id=author_name,
                author_name=author_name,
                timestamp=datetime.now(),  # Instagram doesn't show exact time on posts
                url=post_url,
                likes=likes,
                shares=0,  # Instagram doesn't show shares
                comments=comments,
                media_urls=media_urls,
                hashtags=hashtags,
            )
            
        except Exception as e:
            logger.warning(f"Error parsing Instagram post {post_url}: {e}")
            return None
    
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        Scrape Instagram profile information.
        
        Args:
            user_id: Username (with or without @)
            
        Returns:
            ScrapedUser with profile info
        """
        username = user_id.strip().lstrip('@')
        url = f"{self.BASE_URL}/{username}/"
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="networkidle")
            await asyncio.sleep(2)
            
            # Get display name
            display_name = username
            for selector in ['header h1', 'header h2', 'meta[property="og:title"]']:
                if selector.startswith('meta'):
                    el = await page.query_selector(selector)
                    if el:
                        display_name = await el.get_attribute("content") or username
                else:
                    name_el = await page.query_selector(selector)
                    if name_el:
                        display_name = await name_el.inner_text()
                        if display_name:
                            break
            
            # Get bio
            bio = ""
            bio_el = await page.query_selector('meta[property="og:description"]')
            if bio_el:
                bio = await bio_el.get_attribute("content") or ""
            
            # Get stats (followers, following, posts)
            followers = 0
            following = 0
            post_count = 0
            
            stats_elements = await page.query_selector_all('header section ul li')
            for i, stat_el in enumerate(stats_elements):
                text = await stat_el.inner_text()
                count = self._parse_count(text)
                if i == 0:
                    post_count = count
                elif i == 1:
                    followers = count
                elif i == 2:
                    following = count
            
            return ScrapedUser(
                platform="instagram",
                user_id=username,
                username=username,
                display_name=display_name.strip(),
                bio=bio.strip()[:500],
                profile_url=url,
                followers=followers,
                following=following,
                post_count=post_count,
            )
            
        except Exception as e:
            logger.error(f"Error scraping Instagram profile @{username}: {e}")
            return None
    
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """
        Instagram doesn't have a public trending page.
        Explore page requires login.
        """
        logger.info("Instagram trending requires login - not available via web scraping")
        return []
    
    def _parse_count(self, text: str) -> int:
        """Parse count from text like '1.2K', '3.5M', '1,234'"""
        if not text:
            return 0
        
        text = text.strip().upper()
        
        multipliers = {'K': 1000, 'M': 1000000, 'B': 1000000000}
        
        for suffix, mult in multipliers.items():
            if suffix in text:
                try:
                    num_str = re.search(r'[\d.,]+', text)
                    if num_str:
                        num = float(num_str.group().replace(',', ''))
                        return int(num * mult)
                except ValueError:
                    pass
                return 0
        
        try:
            num_str = re.search(r'[\d,]+', text)
            if num_str:
                return int(num_str.group().replace(',', ''))
        except ValueError:
            pass
        
        return 0
