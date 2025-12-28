"""
Telegram Crawler
Scrapes public Telegram channels via web interface
"""

import re
import asyncio
from typing import List, Optional
from datetime import datetime
from urllib.parse import urljoin

from ...models.pubop import ScrapedPost, ScrapedUser, ScrapedTrend, Platform
from ...utils.logger import get_logger
from .base import BaseCrawler
from .client import LightPandaClient

logger = get_logger('pubop.crawler.telegram')


class TelegramCrawler(BaseCrawler):
    """
    Crawler for Telegram public channels.
    
    Scrapes content from t.me/s/<channel> (public channel preview)
    which doesn't require authentication.
    
    Example:
        async with LightPandaClient() as client:
            crawler = TelegramCrawler(client)
            posts = await crawler.scrape_channel("dulorov", limit=50)
    """
    
    PLATFORM = Platform.TELEGRAM.value
    BASE_URL = "https://t.me/s"
    
    # CSS Selectors for Telegram web preview
    SELECTORS = {
        "post": ".tgme_widget_message",
        "post_text": ".tgme_widget_message_text",
        "post_date": ".tgme_widget_message_date time",
        "post_views": ".tgme_widget_message_views",
        "post_link": ".tgme_widget_message_date",
        "author_name": ".tgme_widget_message_owner_name",
        "channel_info": ".tgme_channel_info",
        "channel_title": ".tgme_channel_info_header_title",
        "channel_description": ".tgme_channel_info_description",
        "channel_counters": ".tgme_channel_info_counter",
        "load_more": ".tme_messages_more",
    }
    
    async def scrape_posts(
        self, 
        query: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape posts from a channel by name.
        
        Args:
            query: Channel username (without @)
            limit: Maximum posts to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        return await self.scrape_channel(query, limit=limit)
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape posts from a public Telegram channel.
        
        Args:
            channel_id: Channel username (without @)
            limit: Maximum posts to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        channel_id = channel_id.lstrip('@')
        url = f"{self.BASE_URL}/{channel_id}"
        posts = []
        
        try:
            page = await self._get_page()
            logger.info(f"Navigating to {url}")
            
            await page.goto(url, wait_until="domcontentloaded")
            await page.wait_for_selector(self.SELECTORS["post"], timeout=10000)
            
            # Scroll to load more posts if needed
            loaded_count = 0
            max_scrolls = 10
            scroll_count = 0
            
            while loaded_count < limit and scroll_count < max_scrolls:
                # Get current posts
                post_elements = await page.query_selector_all(self.SELECTORS["post"])
                loaded_count = len(post_elements)
                
                if loaded_count >= limit:
                    break
                
                # Try to load more
                load_more = await page.query_selector(self.SELECTORS["load_more"])
                if load_more:
                    await load_more.click()
                    await asyncio.sleep(1)
                else:
                    # Scroll down to trigger lazy loading
                    await page.evaluate("window.scrollTo(0, document.body.scrollHeight)")
                    await asyncio.sleep(0.5)
                
                scroll_count += 1
            
            # Extract posts
            post_elements = await page.query_selector_all(self.SELECTORS["post"])
            logger.info(f"Found {len(post_elements)} posts in channel {channel_id}")
            
            for element in post_elements[:limit]:
                post = await self._parse_post(element, channel_id)
                if post:
                    posts.append(post)
            
        except Exception as e:
            logger.error(f"Error scraping Telegram channel {channel_id}: {e}")
            raise
        
        return posts
    
    async def _parse_post(self, element, channel_id: str) -> Optional[ScrapedPost]:
        """Parse a post element into ScrapedPost"""
        try:
            # Get post text
            text_el = await element.query_selector(self.SELECTORS["post_text"])
            content = await text_el.inner_text() if text_el else ""
            
            # Get post ID from data attribute or link
            post_id = await element.get_attribute("data-post") or ""
            if "/" in post_id:
                post_id = post_id.split("/")[-1]
            
            # Get timestamp
            time_el = await element.query_selector(self.SELECTORS["post_date"])
            timestamp = datetime.now()
            if time_el:
                datetime_attr = await time_el.get_attribute("datetime")
                if datetime_attr:
                    try:
                        timestamp = datetime.fromisoformat(datetime_attr.replace("Z", "+00:00"))
                    except ValueError:
                        pass
            
            # Get views
            views = 0
            views_el = await element.query_selector(self.SELECTORS["post_views"])
            if views_el:
                views_text = await views_el.inner_text()
                views = self._parse_count(views_text)
            
            # Get link
            link_el = await element.query_selector(self.SELECTORS["post_link"])
            url = None
            if link_el:
                href = await link_el.get_attribute("href")
                if href:
                    url = href
            
            # Get media
            media_urls = []
            img_elements = await element.query_selector_all("img")
            for img in img_elements:
                src = await img.get_attribute("src")
                if src and "emoji" not in src:
                    media_urls.append(src)
            
            return ScrapedPost(
                platform=self.PLATFORM,
                post_id=post_id or str(hash(content))[:10],
                content=content.strip(),
                author_id=channel_id,
                author_name=channel_id,
                timestamp=timestamp,
                url=url,
                views=views,
                media_urls=media_urls,
            )
            
        except Exception as e:
            logger.warning(f"Error parsing post: {e}")
            return None
    
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        Scrape channel/user profile information.
        
        Args:
            user_id: Channel username
            
        Returns:
            ScrapedUser with channel info
        """
        user_id = user_id.lstrip('@')
        url = f"{self.BASE_URL}/{user_id}"
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="domcontentloaded")
            await page.wait_for_selector(self.SELECTORS["channel_info"], timeout=10000)
            
            # Get channel title
            title_el = await page.query_selector(self.SELECTORS["channel_title"])
            display_name = await title_el.inner_text() if title_el else user_id
            
            # Get description
            desc_el = await page.query_selector(self.SELECTORS["channel_description"])
            bio = await desc_el.inner_text() if desc_el else ""
            
            # Get subscriber count
            followers = 0
            counters = await page.query_selector_all(self.SELECTORS["channel_counters"])
            for counter in counters:
                text = await counter.inner_text()
                if "subscriber" in text.lower() or "member" in text.lower():
                    count_el = await counter.query_selector(".counter_value")
                    if count_el:
                        count_text = await count_el.inner_text()
                        followers = self._parse_count(count_text)
            
            return ScrapedUser(
                platform=self.PLATFORM,
                user_id=user_id,
                username=user_id,
                display_name=display_name.strip(),
                bio=bio.strip(),
                profile_url=url,
                followers=followers,
            )
            
        except Exception as e:
            logger.error(f"Error scraping Telegram user {user_id}: {e}")
            return None
    
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """
        Telegram doesn't have a public trending page.
        Returns empty list.
        """
        logger.info("Telegram trending not available via web scraping")
        return []
    
    def _parse_count(self, text: str) -> int:
        """Parse count string like '1.2K' or '3.5M' to integer"""
        if not text:
            return 0
        
        text = text.strip().upper()
        multipliers = {'K': 1000, 'M': 1000000, 'B': 1000000000}
        
        for suffix, mult in multipliers.items():
            if suffix in text:
                try:
                    num = float(text.replace(suffix, '').strip())
                    return int(num * mult)
                except ValueError:
                    return 0
        
        try:
            return int(re.sub(r'[^\d]', '', text))
        except ValueError:
            return 0
