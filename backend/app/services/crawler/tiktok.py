"""
TikTok Crawler
Scrapes public TikTok profiles and hashtags
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

logger = get_logger('pubop.crawler.tiktok')


class TikTokCrawler(BaseCrawler):
    """
    Crawler for public TikTok profiles and hashtags.
    
    TikTok has strong anti-bot measures. This crawler requires
    Browserless (not LightPanda) due to complex JavaScript.
    
    Example:
        async with LightPandaClient(engine="browserless") as client:
            crawler = TikTokCrawler(client)
            posts = await crawler.scrape_channel("tiktok", limit=20)
    """
    
    PLATFORM = "tiktok"
    BASE_URL = "https://www.tiktok.com"
    
    # Selectors for TikTok
    SELECTORS = {
        "video_item": '[data-e2e="user-post-item"], [class*="DivItemContainer"]',
        "video_link": 'a[href*="/video/"]',
        "video_desc": '[data-e2e="video-desc"], [class*="SpanText"]',
        "video_stats": '[data-e2e="video-views"], [class*="StrongVideoCount"]',
        "user_name": '[data-e2e="user-title"], h1, h2',
        "user_bio": '[data-e2e="user-bio"], [class*="SpanOtherInfos"]',
        "user_stats": '[data-e2e="followers-count"], [data-e2e="likes-count"]',
        "hashtag_video": '[data-e2e="challenge-item"], [class*="DivItemContainer"]',
    }
    
    async def scrape_posts(
        self, 
        query: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Search TikTok by hashtag.
        
        Args:
            query: Hashtag to search (without #)
            limit: Maximum posts to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        hashtag = query.strip().lstrip('#')
        url = f"{self.BASE_URL}/tag/{quote_plus(hashtag)}"
        
        logger.info(f"Searching TikTok hashtag: #{hashtag}")
        return await self._scrape_videos_from_url(url, hashtag, limit)
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape videos from a TikTok user profile.
        
        Args:
            channel_id: Username (with or without @)
            limit: Maximum videos to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        username = channel_id.strip().lstrip('@')
        url = f"{self.BASE_URL}/@{username}"
        
        logger.info(f"Scraping TikTok profile: @{username}")
        return await self._scrape_videos_from_url(url, username, limit)
    
    async def _scrape_videos_from_url(
        self, 
        url: str, 
        source_id: str, 
        limit: int
    ) -> List[ScrapedPost]:
        """Internal method to scrape videos from any TikTok page"""
        posts = []
        
        try:
            page = await self._get_page()
            
            # TikTok may require extra wait time
            await page.goto(url, wait_until="networkidle")
            await asyncio.sleep(4)  # TikTok loads dynamically
            
            # Check for captcha or block
            captcha = await page.query_selector('[class*="captcha"], [class*="Captcha"]')
            if captcha:
                logger.warning("TikTok captcha detected - may need human verification")
            
            # Find video items
            video_elements = []
            for selector in ['[data-e2e="user-post-item"]', 'div[class*="ItemContainer"] a[href*="/video/"]', 'a[href*="/video/"]']:
                video_elements = await page.query_selector_all(selector)
                if video_elements:
                    break
            
            if not video_elements:
                logger.warning(f"No videos found at {url}")
                return posts
            
            logger.info(f"Found {len(video_elements)} video elements")
            
            # Scroll to load more videos
            scroll_count = 0
            while len(video_elements) < limit and scroll_count < 10:
                await page.evaluate("window.scrollBy(0, 1000)")
                await asyncio.sleep(2)
                
                for selector in ['[data-e2e="user-post-item"]', 'a[href*="/video/"]']:
                    video_elements = await page.query_selector_all(selector)
                    if len(video_elements) >= limit:
                        break
                
                scroll_count += 1
            
            # Extract video links and basic info
            video_urls = []
            for el in video_elements[:limit]:
                # Get video URL
                href = await el.get_attribute("href")
                if href and "/video/" in href and href not in video_urls:
                    if not href.startswith("http"):
                        href = f"{self.BASE_URL}{href}"
                    video_urls.append(href)
                
                # Or look for nested link
                if not href or "/video/" not in str(href):
                    link_el = await el.query_selector('a[href*="/video/"]')
                    if link_el:
                        href = await link_el.get_attribute("href")
                        if href and href not in video_urls:
                            if not href.startswith("http"):
                                href = f"{self.BASE_URL}{href}"
                            video_urls.append(href)
            
            logger.info(f"Found {len(video_urls)} unique video URLs")
            
            # Extract basic info from each video (without navigating)
            for i, video_url in enumerate(video_urls[:limit]):
                try:
                    # Extract video ID from URL
                    video_id_match = re.search(r'/video/(\d+)', video_url)
                    video_id = video_id_match.group(1) if video_id_match else str(hash(video_url))[:12]
                    
                    # Extract username from URL
                    username_match = re.search(r'@([^/]+)', video_url)
                    author = username_match.group(1) if username_match else source_id
                    
                    post = ScrapedPost(
                        platform=self.PLATFORM,
                        post_id=video_id,
                        content=f"TikTok video by @{author}",  # Full content requires navigating to video
                        author_id=author,
                        author_name=author,
                        timestamp=datetime.now(),
                        url=video_url,
                        likes=0,
                        shares=0,
                        comments=0,
                        media_urls=[video_url],
                        hashtags=[],
                    )
                    posts.append(post)
                    
                except Exception as e:
                    logger.warning(f"Error extracting video info: {e}")
            
        except Exception as e:
            logger.error(f"Error scraping TikTok: {e}")
            raise
        
        return posts
    
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        Scrape TikTok user profile information.
        
        Args:
            user_id: Username (with or without @)
            
        Returns:
            ScrapedUser with profile info
        """
        username = user_id.strip().lstrip('@')
        url = f"{self.BASE_URL}/@{username}"
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="networkidle")
            await asyncio.sleep(3)
            
            # Get display name
            display_name = username
            for selector in ['h1[data-e2e="user-title"]', 'h2[data-e2e="user-subtitle"]', 'h1', 'h2']:
                name_el = await page.query_selector(selector)
                if name_el:
                    name = await name_el.inner_text()
                    if name and len(name) < 100:
                        display_name = name.strip()
                        break
            
            # Get bio
            bio = ""
            bio_el = await page.query_selector('[data-e2e="user-bio"], [class*="SpanBio"]')
            if bio_el:
                bio = await bio_el.inner_text()
            
            # Get followers
            followers = 0
            followers_el = await page.query_selector('[data-e2e="followers-count"], [title*="Followers"]')
            if followers_el:
                text = await followers_el.inner_text()
                followers = self._parse_count(text)
            
            # Get following
            following = 0
            following_el = await page.query_selector('[data-e2e="following-count"], [title*="Following"]')
            if following_el:
                text = await following_el.inner_text()
                following = self._parse_count(text)
            
            # Get likes
            likes = 0
            likes_el = await page.query_selector('[data-e2e="likes-count"], [title*="Likes"]')
            if likes_el:
                text = await likes_el.inner_text()
                likes = self._parse_count(text)
            
            return ScrapedUser(
                platform=self.PLATFORM,
                user_id=username,
                username=username,
                display_name=display_name,
                bio=bio.strip()[:500],
                profile_url=url,
                followers=followers,
                following=following,
                post_count=0,  # Would need to count videos
            )
            
        except Exception as e:
            logger.error(f"Error scraping TikTok profile @{username}: {e}")
            return None
    
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """
        Scrape TikTok trending/discover page.
        Note: TikTok trending requires login in most regions.
        """
        logger.info("TikTok trending requires login - limited access")
        return []
    
    def _parse_count(self, text: str) -> int:
        """Parse count from text like '1.2K', '3.5M', '1234'"""
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
