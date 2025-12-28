"""
Facebook Crawler
Scrapes public Facebook pages and groups
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

logger = get_logger('pubop.crawler.facebook')


class FacebookCrawler(BaseCrawler):
    """
    Crawler for Facebook public pages and groups.
    
    Scrapes content from m.facebook.com (mobile version) which is 
    lighter and more scraping-friendly than the desktop version.
    
    Note: Facebook has strong anti-scraping measures. This crawler
    works best for public pages. For more reliable access, consider
    using the Facebook Graph API with proper authorization.
    
    Example:
        async with LightPandaClient() as client:
            crawler = FacebookCrawler(client)
            posts = await crawler.scrape_channel("BBCNews", limit=50)
    """
    
    PLATFORM = Platform.FACEBOOK.value
    BASE_URL = "https://m.facebook.com"
    DESKTOP_URL = "https://www.facebook.com"
    
    # Selectors for mobile Facebook
    SELECTORS = {
        "post": '[data-ft*="story"], article, [role="article"]',
        "post_text": '[data-gt*="message"], [data-ad-preview="message"]',
        "post_time": "abbr[data-utime], abbr[data-store]",
        "post_reactions": '[data-sigil="reactions-sentence-container"]',
        "page_name": 'h1, [data-pagelet*="ProfileName"]',
        "page_about": '[data-pagelet*="ProfileAbout"]',
        "page_likes": '[data-pagelet*="ProfileLikes"]',
    }
    
    async def scrape_posts(
        self, 
        query: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Search Facebook for posts matching query.
        
        Note: Facebook search requires login for full results.
        This returns limited public results.
        
        Args:
            query: Search query
            limit: Maximum posts to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        # Facebook search is restricted - redirect to page scraping
        logger.info(f"Facebook search not available without login. Query: {query}")
        logger.info("Tip: Use scrape_channel() with a page name instead")
        return []
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape posts from a public Facebook page.
        
        Args:
            channel_id: Page username or ID
            limit: Maximum posts to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        page_id = channel_id.strip().lstrip('/')
        url = f"{self.BASE_URL}/{page_id}"
        posts = []
        
        try:
            page = await self._get_page()
            logger.info(f"Scraping Facebook page: {page_id}")
            
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(2)  # Allow dynamic content to load
            
            # Try multiple selectors for posts
            post_elements = []
            for selector in ['article', '[role="article"]', '[data-ft]']:
                post_elements = await page.query_selector_all(selector)
                if post_elements:
                    break
            
            if not post_elements:
                logger.warning(f"No posts found on page {page_id}")
                return posts
            
            # Scroll to load more posts
            for _ in range(min(limit // 5, 10)):
                if len(post_elements) >= limit:
                    break
                
                await page.evaluate("window.scrollBy(0, 1000)")
                await asyncio.sleep(2)
                
                for selector in ['article', '[role="article"]', '[data-ft]']:
                    post_elements = await page.query_selector_all(selector)
                    if post_elements:
                        break
            
            logger.info(f"Found {len(post_elements)} posts on page {page_id}")
            
            for element in post_elements[:limit]:
                post = await self._parse_post(element, page_id)
                if post and post.content:  # Only add posts with content
                    posts.append(post)
            
        except Exception as e:
            logger.error(f"Error scraping Facebook page {page_id}: {e}")
            raise
        
        return posts
    
    async def _parse_post(self, element, page_id: str) -> Optional[ScrapedPost]:
        """Parse a Facebook post element"""
        try:
            # Get post text - try multiple selectors
            content = ""
            for selector in ['[data-gt]', '[data-ad-preview]', 'p', '[dir="auto"]']:
                text_elements = await element.query_selector_all(selector)
                for text_el in text_elements:
                    text = await text_el.inner_text()
                    if text and len(text) > len(content):
                        content = text
            
            if not content.strip():
                return None
            
            # Generate post ID from content hash
            post_id = str(hash(content + page_id))[:12]
            
            # Get timestamp
            timestamp = datetime.now()
            time_el = await element.query_selector('abbr, time, [data-utime]')
            if time_el:
                utime = await time_el.get_attribute("data-utime")
                if utime:
                    try:
                        timestamp = datetime.fromtimestamp(int(utime))
                    except (ValueError, OSError):
                        pass
            
            # Get reactions/likes count
            likes = 0
            reactions_el = await element.query_selector('[aria-label*="reaction"], [data-sigil*="reaction"]')
            if reactions_el:
                aria_label = await reactions_el.get_attribute("aria-label") or ""
                likes = self._parse_count(aria_label)
            
            # Get shares count
            shares = 0
            share_el = await element.query_selector('[data-sigil*="share"]')
            if share_el:
                share_text = await share_el.inner_text()
                shares = self._parse_count(share_text)
            
            # Get comments count
            comments = 0
            comment_el = await element.query_selector('[data-sigil*="comment"]')
            if comment_el:
                comment_text = await comment_el.inner_text()
                comments = self._parse_count(comment_text)
            
            # Get media
            media_urls = []
            for selector in ['img[src*="fbcdn"]', 'img[src*="scontent"]']:
                img_elements = await element.query_selector_all(selector)
                for img in img_elements[:3]:  # Limit media
                    src = await img.get_attribute("src")
                    if src and "emoji" not in src.lower():
                        media_urls.append(src)
            
            # Extract hashtags
            hashtags = re.findall(r'#(\w+)', content)
            
            return ScrapedPost(
                platform=self.PLATFORM,
                post_id=post_id,
                content=content.strip()[:2000],  # Limit content length
                author_id=page_id,
                author_name=page_id,
                timestamp=timestamp,
                url=f"{self.DESKTOP_URL}/{page_id}",
                likes=likes,
                shares=shares,
                comments=comments,
                media_urls=media_urls,
                hashtags=hashtags,
            )
            
        except Exception as e:
            logger.warning(f"Error parsing Facebook post: {e}")
            return None
    
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        Scrape Facebook page/profile information.
        
        Args:
            user_id: Page username or ID
            
        Returns:
            ScrapedUser with page info
        """
        page_id = user_id.strip().lstrip('/')
        url = f"{self.BASE_URL}/{page_id}"
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(2)
            
            # Get page name
            display_name = page_id
            for selector in ['h1', '[role="heading"]', 'title']:
                name_el = await page.query_selector(selector)
                if name_el:
                    name_text = await name_el.inner_text()
                    if name_text and len(name_text) < 100:
                        display_name = name_text.strip()
                        break
            
            # Get description/about
            bio = ""
            for selector in ['[data-pagelet*="About"]', '.bio', '[role="complementary"]']:
                bio_el = await page.query_selector(selector)
                if bio_el:
                    bio = await bio_el.inner_text()
                    if bio:
                        bio = bio[:500]  # Limit length
                        break
            
            # Get likes/followers count
            followers = 0
            for selector in ['[data-pagelet*="Likes"]', '[aria-label*="like"]', '[aria-label*="follower"]']:
                likes_el = await page.query_selector(selector)
                if likes_el:
                    text = await likes_el.inner_text()
                    if text:
                        followers = self._parse_count(text)
                        if followers > 0:
                            break
            
            return ScrapedUser(
                platform=self.PLATFORM,
                user_id=page_id,
                username=page_id,
                display_name=display_name,
                bio=bio.strip(),
                profile_url=f"{self.DESKTOP_URL}/{page_id}",
                followers=followers,
            )
            
        except Exception as e:
            logger.error(f"Error scraping Facebook page {page_id}: {e}")
            return None
    
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """
        Facebook doesn't have a public trending page.
        Returns empty list.
        """
        logger.info("Facebook trending not available via web scraping")
        return []
    
    def _parse_count(self, text: str) -> int:
        """Parse count from text like '1.2K likes' or '3.5M followers'"""
        if not text:
            return 0
        
        text = text.strip().upper()
        
        # Handle formats like "1,234", "1.2K", "3.5M"
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
        
        # Plain number
        try:
            num_str = re.search(r'[\d,]+', text)
            if num_str:
                return int(num_str.group().replace(',', ''))
        except ValueError:
            pass
        
        return 0
