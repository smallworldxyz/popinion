"""
Twitter/X Crawler
Scrapes public tweets from X.com (formerly Twitter)
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

logger = get_logger('pubop.crawler.twitter')


class TwitterCrawler(BaseCrawler):
    """
    Crawler for Twitter/X public content.
    
    Scrapes tweets from x.com search results and user profiles.
    No API key required - uses web scraping.
    
    Note: X.com has aggressive anti-scraping measures. This crawler
    may need adjustments as the site changes.
    
    Example:
        async with LightPandaClient() as client:
            crawler = TwitterCrawler(client)
            posts = await crawler.scrape_posts("python programming", limit=50)
    """
    
    PLATFORM = Platform.TWITTER.value
    BASE_URL = "https://x.com"
    SEARCH_URL = "https://x.com/search"
    
    # CSS Selectors - X uses dynamic class names, so we use data attributes
    SELECTORS = {
        "tweet": 'article[data-testid="tweet"]',
        "tweet_text": '[data-testid="tweetText"]',
        "tweet_user": '[data-testid="User-Name"]',
        "tweet_time": "time",
        "tweet_stats": '[data-testid="app-text-transition-container"]',
        "user_name": '[data-testid="UserName"]',
        "user_bio": '[data-testid="UserDescription"]',
        "user_stats": '[data-testid="UserProfileHeader_Items"]',
        "trending_item": '[data-testid="trend"]',
    }
    
    async def scrape_posts(
        self, 
        query: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Search and scrape tweets matching a query.
        
        Args:
            query: Search query string
            limit: Maximum tweets to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        encoded_query = quote_plus(query)
        url = f"{self.SEARCH_URL}?q={encoded_query}&src=typed_query&f=live"
        posts = []
        
        try:
            page = await self._get_page()
            logger.info(f"Searching Twitter for: {query}")
            
            # Navigate to search
            await page.goto(url, wait_until="domcontentloaded")
            
            # Wait for tweets to load (with timeout)
            try:
                await page.wait_for_selector(self.SELECTORS["tweet"], timeout=15000)
            except Exception:
                logger.warning("No tweets found or page didn't load properly")
                return posts
            
            # Scroll to load more tweets
            loaded_count = 0
            max_scrolls = min(limit // 5, 20)  # Limit scrolling
            
            for _ in range(max_scrolls):
                tweet_elements = await page.query_selector_all(self.SELECTORS["tweet"])
                loaded_count = len(tweet_elements)
                
                if loaded_count >= limit:
                    break
                
                # Scroll down
                await page.evaluate("window.scrollBy(0, 800)")
                await asyncio.sleep(1.5)  # Twitter rate limits
            
            # Extract tweets
            tweet_elements = await page.query_selector_all(self.SELECTORS["tweet"])
            logger.info(f"Found {len(tweet_elements)} tweets for query: {query}")
            
            for element in tweet_elements[:limit]:
                post = await self._parse_tweet(element)
                if post:
                    posts.append(post)
            
        except Exception as e:
            logger.error(f"Error searching Twitter for '{query}': {e}")
            raise
        
        return posts
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape tweets from a user's profile.
        
        Args:
            channel_id: Twitter username (without @)
            limit: Maximum tweets to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        username = channel_id.lstrip('@')
        url = f"{self.BASE_URL}/{username}"
        posts = []
        
        try:
            page = await self._get_page()
            logger.info(f"Scraping Twitter profile: {username}")
            
            await page.goto(url, wait_until="domcontentloaded")
            
            try:
                await page.wait_for_selector(self.SELECTORS["tweet"], timeout=15000)
            except Exception:
                logger.warning(f"No tweets found for user {username}")
                return posts
            
            # Scroll to load more
            for _ in range(min(limit // 5, 10)):
                tweet_elements = await page.query_selector_all(self.SELECTORS["tweet"])
                if len(tweet_elements) >= limit:
                    break
                
                await page.evaluate("window.scrollBy(0, 800)")
                await asyncio.sleep(1.5)
            
            # Extract tweets
            tweet_elements = await page.query_selector_all(self.SELECTORS["tweet"])
            
            for element in tweet_elements[:limit]:
                post = await self._parse_tweet(element, default_author=username)
                if post:
                    posts.append(post)
            
        except Exception as e:
            logger.error(f"Error scraping Twitter user {username}: {e}")
            raise
        
        return posts
    
    async def _parse_tweet(
        self, 
        element, 
        default_author: Optional[str] = None
    ) -> Optional[ScrapedPost]:
        """Parse a tweet element into ScrapedPost"""
        try:
            # Get tweet text
            text_el = await element.query_selector(self.SELECTORS["tweet_text"])
            content = await text_el.inner_text() if text_el else ""
            
            # Get author info
            author_id = default_author or "unknown"
            author_name = default_author or "Unknown"
            
            user_el = await element.query_selector(self.SELECTORS["tweet_user"])
            if user_el:
                user_text = await user_el.inner_text()
                # Parse "Display Name\n@username" format
                parts = user_text.split('\n')
                if len(parts) >= 2:
                    author_name = parts[0].strip()
                    author_id = parts[1].replace('@', '').strip()
                elif len(parts) == 1:
                    author_name = parts[0].strip()
            
            # Get timestamp
            timestamp = datetime.now()
            time_el = await element.query_selector(self.SELECTORS["tweet_time"])
            if time_el:
                datetime_attr = await time_el.get_attribute("datetime")
                if datetime_attr:
                    try:
                        timestamp = datetime.fromisoformat(datetime_attr.replace("Z", "+00:00"))
                    except ValueError:
                        pass
            
            # Get post ID from aria-labelledby or generate from content
            post_id = str(hash(content + author_id))[:12]
            
            # Get engagement stats
            likes, shares = 0, 0
            stats = await element.query_selector_all(self.SELECTORS["tweet_stats"])
            if len(stats) >= 3:
                # Order is typically: replies, retweets, likes
                shares = self._parse_count(await stats[1].inner_text()) if len(stats) > 1 else 0
                likes = self._parse_count(await stats[2].inner_text()) if len(stats) > 2 else 0
            
            # Get media
            media_urls = []
            img_elements = await element.query_selector_all('img[src*="media"]')
            for img in img_elements:
                src = await img.get_attribute("src")
                if src:
                    media_urls.append(src)
            
            # Extract hashtags
            hashtags = re.findall(r'#(\w+)', content)
            
            # Extract mentions
            mentions = re.findall(r'@(\w+)', content)
            
            return ScrapedPost(
                platform=self.PLATFORM,
                post_id=post_id,
                content=content.strip(),
                author_id=author_id,
                author_name=author_name,
                timestamp=timestamp,
                url=f"{self.BASE_URL}/{author_id}/status/{post_id}",
                likes=likes,
                shares=shares,
                media_urls=media_urls,
                hashtags=hashtags,
                mentions=mentions,
            )
            
        except Exception as e:
            logger.warning(f"Error parsing tweet: {e}")
            return None
    
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        Scrape Twitter user profile.
        
        Args:
            user_id: Twitter username
            
        Returns:
            ScrapedUser with profile info
        """
        username = user_id.lstrip('@')
        url = f"{self.BASE_URL}/{username}"
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="domcontentloaded")
            
            # Wait for profile to load
            await asyncio.sleep(2)
            
            # Get display name
            display_name = username
            name_el = await page.query_selector('[data-testid="UserName"]')
            if name_el:
                name_text = await name_el.inner_text()
                display_name = name_text.split('\n')[0] if '\n' in name_text else name_text
            
            # Get bio
            bio = ""
            bio_el = await page.query_selector('[data-testid="UserDescription"]')
            if bio_el:
                bio = await bio_el.inner_text()
            
            # Get follower/following counts from profile
            followers, following = 0, 0
            
            # Look for links containing "followers" and "following"
            follower_links = await page.query_selector_all('a[href*="followers"], a[href*="following"]')
            for link in follower_links:
                text = await link.inner_text()
                href = await link.get_attribute("href") or ""
                
                if "followers" in href.lower():
                    followers = self._parse_count(text)
                elif "following" in href.lower():
                    following = self._parse_count(text)
            
            return ScrapedUser(
                platform=self.PLATFORM,
                user_id=username,
                username=username,
                display_name=display_name.strip(),
                bio=bio.strip(),
                profile_url=url,
                followers=followers,
                following=following,
            )
            
        except Exception as e:
            logger.error(f"Error scraping Twitter user {username}: {e}")
            return None
    
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """
        Scrape trending topics from Twitter.
        
        Returns:
            List of ScrapedTrend objects
        """
        trends = []
        url = f"{self.BASE_URL}/explore/tabs/trending"
        
        try:
            page = await self._get_page()
            await page.goto(url, wait_until="domcontentloaded")
            
            # Wait for trends to load
            try:
                await page.wait_for_selector(self.SELECTORS["trending_item"], timeout=10000)
            except Exception:
                logger.warning("Trending topics not available")
                return trends
            
            trend_elements = await page.query_selector_all(self.SELECTORS["trending_item"])
            
            for element in trend_elements[:20]:
                try:
                    text = await element.inner_text()
                    lines = text.strip().split('\n')
                    
                    # Find the topic (usually the main text)
                    topic = lines[0] if lines else ""
                    
                    # Try to find post count
                    post_count = 0
                    for line in lines:
                        if any(x in line.lower() for x in ['posts', 'tweets', 'k posts', 'm posts']):
                            post_count = self._parse_count(line)
                            break
                    
                    if topic:
                        trends.append(ScrapedTrend(
                            platform=self.PLATFORM,
                            topic=topic.strip(),
                            post_count=post_count,
                            timestamp=datetime.now(),
                        ))
                except Exception:
                    continue
            
        except Exception as e:
            logger.error(f"Error scraping Twitter trends: {e}")
        
        return trends
    
    def _parse_count(self, text: str) -> int:
        """Parse count string like '1.2K' or '3.5M' to integer"""
        if not text:
            return 0
        
        text = text.strip().upper()
        
        # Handle formats like "1,234" or "1.2K" or "3.5M"
        multipliers = {'K': 1000, 'M': 1000000, 'B': 1000000000}
        
        for suffix, mult in multipliers.items():
            if suffix in text:
                try:
                    # Extract number before suffix
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
