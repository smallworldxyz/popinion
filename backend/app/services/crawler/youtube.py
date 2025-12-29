"""
YouTube Crawler
Scrapes public YouTube channels and video metadata
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

logger = get_logger('pubop.crawler.youtube')


class YouTubeCrawler(BaseCrawler):
    """
    Crawler for public YouTube channels and videos.
    
    Scrapes video metadata, view counts, and comments from
    YouTube without API authentication.
    
    Example:
        async with LightPandaClient(engine="browserless") as client:
            crawler = YouTubeCrawler(client)
            posts = await crawler.scrape_channel("YouTube", limit=20)
    """
    
    PLATFORM = "youtube"
    BASE_URL = "https://www.youtube.com"
    
    # Selectors for YouTube
    SELECTORS = {
        "video_item": 'ytd-rich-item-renderer, ytd-video-renderer',
        "video_link": 'a#video-title-link, a#video-title',
        "video_title": '#video-title',
        "video_views": '#metadata-line span:first-child',
        "video_date": '#metadata-line span:nth-child(2)',
        "channel_name": '#channel-name, ytd-channel-name yt-formatted-string',
        "channel_subs": '#subscriber-count, #subscribers',
        "shorts_item": 'ytd-reel-item-renderer',
    }
    
    async def scrape_posts(
        self, 
        query: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Search YouTube for videos matching query.
        
        Args:
            query: Search query
            limit: Maximum videos to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        url = f"{self.BASE_URL}/results?search_query={quote_plus(query)}"
        
        logger.info(f"Searching YouTube: {query}")
        return await self._scrape_videos_from_url(url, query, limit)
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape videos from a YouTube channel.
        
        Args:
            channel_id: Channel handle (with or without @) or channel name
            limit: Maximum videos to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        # Handle both @handle and plain name
        channel = channel_id.strip().lstrip('@')
        
        # Try @handle first (new format)
        url = f"{self.BASE_URL}/@{channel}/videos"
        
        logger.info(f"Scraping YouTube channel: @{channel}")
        posts = await self._scrape_videos_from_url(url, channel, limit)
        
        # If no results, try /c/ or /user/ format
        if not posts:
            logger.info(f"Trying alternative URL format for {channel}")
            url = f"{self.BASE_URL}/c/{channel}/videos"
            posts = await self._scrape_videos_from_url(url, channel, limit)
        
        return posts
    
    async def _scrape_videos_from_url(
        self, 
        url: str, 
        source_id: str, 
        limit: int
    ) -> List[ScrapedPost]:
        """Internal method to scrape videos from any YouTube page"""
        posts = []
        
        try:
            page = await self._get_page()
            
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(3)  # YouTube loads dynamically
            
            # Accept cookies if dialog appears
            try:
                accept_btn = await page.query_selector('button[aria-label*="Accept"], tp-yt-paper-button:has-text("Accept")')
                if accept_btn:
                    await accept_btn.click()
                    await asyncio.sleep(1)
            except:
                pass
            
            # Find video elements
            video_elements = []
            for selector in ['ytd-rich-item-renderer', 'ytd-video-renderer', 'ytd-grid-video-renderer']:
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
                
                for selector in ['ytd-rich-item-renderer', 'ytd-video-renderer']:
                    video_elements = await page.query_selector_all(selector)
                    if len(video_elements) >= limit:
                        break
                
                scroll_count += 1
            
            # Extract video info
            for element in video_elements[:limit]:
                post = await self._parse_video(element, source_id)
                if post:
                    posts.append(post)
            
        except Exception as e:
            logger.error(f"Error scraping YouTube: {e}")
            raise
        
        return posts
    
    async def _parse_video(self, element, source_id: str) -> Optional[ScrapedPost]:
        """Parse a YouTube video element"""
        try:
            # Get video link and title
            title = ""
            video_url = ""
            
            link_el = await element.query_selector('a#video-title-link, a#video-title, a#thumbnail')
            if link_el:
                href = await link_el.get_attribute("href")
                if href:
                    video_url = f"{self.BASE_URL}{href}" if not href.startswith("http") else href
                
                title = await link_el.get_attribute("title") or ""
                if not title:
                    title = await link_el.inner_text()
            
            if not video_url or not title:
                return None
            
            # Extract video ID
            video_id_match = re.search(r'[?&]v=([^&]+)', video_url)
            if video_id_match:
                video_id = video_id_match.group(1)
            else:
                video_id_match = re.search(r'/shorts/([^?]+)', video_url)
                video_id = video_id_match.group(1) if video_id_match else str(hash(video_url))[:12]
            
            # Get view count
            views = 0
            views_el = await element.query_selector('#metadata-line span')
            if views_el:
                views_text = await views_el.inner_text()
                views = self._parse_count(views_text)
            
            # Get channel name
            channel_name = source_id
            channel_el = await element.query_selector('#channel-name yt-formatted-string, #channel-name a')
            if channel_el:
                channel_name = await channel_el.inner_text()
            
            # Get thumbnail
            media_urls = []
            thumb_el = await element.query_selector('img#img')
            if thumb_el:
                src = await thumb_el.get_attribute("src")
                if src and "ytimg.com" in src:
                    media_urls.append(src)
            
            return ScrapedPost(
                platform=self.PLATFORM,
                post_id=video_id,
                content=title.strip()[:2000],
                author_id=source_id,
                author_name=channel_name.strip() if channel_name else source_id,
                timestamp=datetime.now(),  # YouTube doesn't show exact time
                url=video_url,
                likes=0,  # Would need to visit video page
                shares=0,
                comments=0,
                views=views,
                media_urls=media_urls,
                hashtags=[],
            )
            
        except Exception as e:
            logger.warning(f"Error parsing YouTube video: {e}")
            return None
    
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        Scrape YouTube channel information.
        
        Args:
            user_id: Channel handle (with or without @)
            
        Returns:
            ScrapedUser with channel info
        """
        channel = user_id.strip().lstrip('@')
        url = f"{self.BASE_URL}/@{channel}"
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(2)
            
            # Get channel name
            display_name = channel
            name_el = await page.query_selector('#channel-name yt-formatted-string, ytd-channel-name yt-formatted-string')
            if name_el:
                display_name = await name_el.inner_text()
            
            # Get description
            bio = ""
            desc_el = await page.query_selector('#description-container, #channel-tagline')
            if desc_el:
                bio = await desc_el.inner_text()
            
            # Get subscriber count
            followers = 0
            subs_el = await page.query_selector('#subscriber-count, #subscribers')
            if subs_el:
                subs_text = await subs_el.inner_text()
                followers = self._parse_count(subs_text)
            
            return ScrapedUser(
                platform=self.PLATFORM,
                user_id=channel,
                username=channel,
                display_name=display_name.strip(),
                bio=bio.strip()[:500],
                profile_url=url,
                followers=followers,
                following=0,  # YouTube doesn't show subscriptions publicly
                post_count=0,
            )
            
        except Exception as e:
            logger.error(f"Error scraping YouTube channel @{channel}: {e}")
            return None
    
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """
        Scrape YouTube trending page.
        """
        url = f"{self.BASE_URL}/feed/trending"
        trends = []
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(2)
            
            # Get trending videos
            video_elements = await page.query_selector_all('ytd-video-renderer')
            
            for i, element in enumerate(video_elements[:10]):
                link_el = await element.query_selector('a#video-title')
                if link_el:
                    title = await link_el.get_attribute("title") or await link_el.inner_text()
                    
                    trends.append(ScrapedTrend(
                        platform=self.PLATFORM,
                        name=title.strip()[:100],
                        url=f"{self.BASE_URL}/feed/trending",
                        rank=i + 1,
                    ))
            
        except Exception as e:
            logger.error(f"Error scraping YouTube trending: {e}")
        
        return trends
    
    def _parse_count(self, text: str) -> int:
        """Parse count from text like '1.2K views', '3.5M subscribers'"""
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
