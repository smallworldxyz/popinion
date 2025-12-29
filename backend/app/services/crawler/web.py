"""
Generic Web Crawler
Scrapes articles and content from any URL (news sites, blogs, etc.)
"""

import re
import asyncio
from typing import List, Optional, Dict, Any
from datetime import datetime
from urllib.parse import urlparse, urljoin

from ...models.pubop import ScrapedPost, ScrapedUser, ScrapedTrend, Platform
from ...utils.logger import get_logger
from .base import BaseCrawler
from .client import LightPandaClient

logger = get_logger('pubop.crawler.web')


class GenericWebCrawler(BaseCrawler):
    """
    Generic web crawler for any URL - news sites, blogs, articles.
    
    Extracts article content, metadata, and links from any webpage.
    Particularly useful for:
    - News sites (Bangkok Post, VnExpress, Kompas, etc.)
    - Blog posts
    - Press releases
    - Government announcements
    
    Example:
        async with LightPandaClient() as client:
            crawler = GenericWebCrawler(client)
            
            # Scrape single article
            posts = await crawler.scrape_url("https://example.com/article")
            
            # Scrape multiple articles from homepage
            posts = await crawler.scrape_channel("https://news.example.com", limit=10)
    """
    
    PLATFORM = "web"
    
    # Common article selectors (tried in order)
    ARTICLE_SELECTORS = [
        'article',
        '[role="article"]',
        '.article',
        '.post',
        '.entry-content',
        '.story',
        '.news-item',
        'main',
        '#content',
        '.content',
    ]
    
    # Common title selectors
    TITLE_SELECTORS = [
        'h1',
        'article h1',
        '.article-title',
        '.post-title',
        '.headline',
        '[itemprop="headline"]',
        'meta[property="og:title"]',
    ]
    
    # Common content selectors
    CONTENT_SELECTORS = [
        'article p',
        '.article-content p',
        '.post-content p',
        '.entry-content p',
        '.story-body p',
        'main p',
        '#content p',
    ]
    
    # Common date selectors
    DATE_SELECTORS = [
        'time',
        '[datetime]',
        '.date',
        '.published',
        '.post-date',
        '[itemprop="datePublished"]',
        'meta[property="article:published_time"]',
    ]
    
    # Common author selectors
    AUTHOR_SELECTORS = [
        '[rel="author"]',
        '.author',
        '.byline',
        '[itemprop="author"]',
        'meta[name="author"]',
    ]
    
    async def scrape_posts(
        self, 
        query: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape articles from a URL or domain.
        
        Args:
            query: URL to scrape (full URL or domain)
            limit: Maximum articles to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        # Normalize URL
        url = query.strip()
        if not url.startswith('http'):
            url = f"https://{url}"
        
        logger.info(f"Scraping web content from: {url}")
        return await self._scrape_page(url, limit)
    
    async def scrape_channel(
        self, 
        channel_id: str, 
        limit: int = 100
    ) -> List[ScrapedPost]:
        """
        Scrape articles from a website (homepage or section).
        
        Finds article links on the page and extracts content from each.
        
        Args:
            channel_id: Website URL
            limit: Maximum articles to retrieve
            
        Returns:
            List of ScrapedPost objects
        """
        # Normalize URL
        url = channel_id.strip()
        if not url.startswith('http'):
            url = f"https://{url}"
        
        logger.info(f"Scraping articles from: {url}")
        return await self._scrape_page(url, limit)
    
    async def scrape_url(
        self, 
        url: str
    ) -> Optional[ScrapedPost]:
        """
        Scrape a single article/page URL.
        
        Args:
            url: Full URL to scrape
            
        Returns:
            ScrapedPost with article content
        """
        posts = await self._scrape_page(url, limit=1, single_page=True)
        return posts[0] if posts else None
    
    async def _scrape_page(
        self, 
        url: str, 
        limit: int,
        single_page: bool = False
    ) -> List[ScrapedPost]:
        """Internal method to scrape a page"""
        posts = []
        
        try:
            page = await self._get_page()
            
            await page.goto(url, wait_until="domcontentloaded")
            await asyncio.sleep(2)
            
            domain = urlparse(url).netloc
            
            if single_page:
                # Scrape just this page as an article
                post = await self._extract_article(page, url, domain)
                if post:
                    posts.append(post)
            else:
                # Find article links on the page
                article_links = await self._find_article_links(page, url)
                
                if not article_links:
                    # If no links found, treat the page itself as an article
                    logger.info("No article links found, treating page as single article")
                    post = await self._extract_article(page, url, domain)
                    if post:
                        posts.append(post)
                else:
                    logger.info(f"Found {len(article_links)} article links")
                    
                    # Scrape each article
                    for article_url in article_links[:limit]:
                        try:
                            await page.goto(article_url, wait_until="domcontentloaded")
                            await asyncio.sleep(1)
                            
                            post = await self._extract_article(page, article_url, domain)
                            if post:
                                posts.append(post)
                        except Exception as e:
                            logger.warning(f"Error scraping {article_url}: {e}")
            
        except Exception as e:
            logger.error(f"Error scraping {url}: {e}")
            raise
        
        return posts
    
    async def _find_article_links(self, page, base_url: str) -> List[str]:
        """Find article links on a page"""
        links = []
        base_domain = urlparse(base_url).netloc
        
        # Common patterns for article URLs
        article_patterns = [
            r'/article/',
            r'/news/',
            r'/story/',
            r'/post/',
            r'/\d{4}/\d{2}/',  # Date-based URLs
            r'\.html$',
            r'/p/',
        ]
        
        # Get all links
        link_elements = await page.query_selector_all('a[href]')
        
        for el in link_elements:
            try:
                href = await el.get_attribute("href")
                if not href:
                    continue
                
                # Make absolute URL
                if not href.startswith('http'):
                    href = urljoin(base_url, href)
                
                # Skip external links
                if urlparse(href).netloc != base_domain:
                    continue
                
                # Skip non-article URLs
                if any(skip in href.lower() for skip in ['login', 'signup', 'contact', 'about', 'privacy', 'terms', '#', 'javascript:']):
                    continue
                
                # Check if it looks like an article
                if any(re.search(pattern, href) for pattern in article_patterns):
                    if href not in links:
                        links.append(href)
                
                # Also check if link is inside article containers
                parent = await el.evaluate("el => el.closest('article, .article, .post, .news-item')")
                if parent and href not in links:
                    links.append(href)
                    
            except Exception:
                continue
        
        return links[:50]  # Limit to prevent too many
    
    async def _extract_article(self, page, url: str, domain: str) -> Optional[ScrapedPost]:
        """Extract article content from a page"""
        try:
            # Get title
            title = await self._extract_title(page)
            if not title:
                title = await page.title() or url
            
            # Get content
            content = await self._extract_content(page)
            if not content:
                content = title
            
            # Get author
            author = await self._extract_author(page) or domain
            
            # Get date
            timestamp = await self._extract_date(page)
            
            # Get images
            media_urls = await self._extract_images(page)
            
            # Extract hashtags/keywords
            hashtags = await self._extract_keywords(page, content)
            
            # Generate post ID from URL
            post_id = str(hash(url))[:12]
            
            return ScrapedPost(
                platform=self.PLATFORM,
                post_id=post_id,
                content=f"{title}\n\n{content}"[:5000],  # Limit content length
                author_id=domain,
                author_name=author,
                timestamp=timestamp,
                url=url,
                likes=0,
                shares=0,
                comments=0,
                media_urls=media_urls[:5],
                hashtags=hashtags[:10],
            )
            
        except Exception as e:
            logger.warning(f"Error extracting article from {url}: {e}")
            return None
    
    async def _extract_title(self, page) -> str:
        """Extract article title"""
        for selector in self.TITLE_SELECTORS:
            try:
                if selector.startswith('meta'):
                    el = await page.query_selector(selector)
                    if el:
                        return await el.get_attribute("content") or ""
                else:
                    el = await page.query_selector(selector)
                    if el:
                        text = await el.inner_text()
                        if text and len(text) > 5:
                            return text.strip()
            except:
                continue
        return ""
    
    async def _extract_content(self, page) -> str:
        """Extract article content"""
        content_parts = []
        
        for selector in self.CONTENT_SELECTORS:
            try:
                elements = await page.query_selector_all(selector)
                for el in elements[:20]:  # Limit paragraphs
                    text = await el.inner_text()
                    if text and len(text) > 20:
                        content_parts.append(text.strip())
                
                if content_parts:
                    break
            except:
                continue
        
        return "\n\n".join(content_parts)
    
    async def _extract_author(self, page) -> str:
        """Extract author name"""
        for selector in self.AUTHOR_SELECTORS:
            try:
                if selector.startswith('meta'):
                    el = await page.query_selector(selector)
                    if el:
                        return await el.get_attribute("content") or ""
                else:
                    el = await page.query_selector(selector)
                    if el:
                        text = await el.inner_text()
                        if text and len(text) < 100:
                            return text.strip()
            except:
                continue
        return ""
    
    async def _extract_date(self, page) -> datetime:
        """Extract publication date"""
        for selector in self.DATE_SELECTORS:
            try:
                if selector.startswith('meta'):
                    el = await page.query_selector(selector)
                    if el:
                        date_str = await el.get_attribute("content")
                        if date_str:
                            return self._parse_date(date_str)
                else:
                    el = await page.query_selector(selector)
                    if el:
                        # Try datetime attribute first
                        dt = await el.get_attribute("datetime")
                        if dt:
                            return self._parse_date(dt)
                        # Then try text content
                        text = await el.inner_text()
                        if text:
                            return self._parse_date(text)
            except:
                continue
        return datetime.now()
    
    def _parse_date(self, date_str: str) -> datetime:
        """Parse date string to datetime"""
        import dateutil.parser
        try:
            return dateutil.parser.parse(date_str)
        except:
            return datetime.now()
    
    async def _extract_images(self, page) -> List[str]:
        """Extract main images"""
        images = []
        
        selectors = [
            'article img[src]',
            '.article-image img',
            '.featured-image img',
            'meta[property="og:image"]',
            'img[src*="content"]',
        ]
        
        for selector in selectors:
            try:
                if 'meta' in selector:
                    el = await page.query_selector(selector)
                    if el:
                        src = await el.get_attribute("content")
                        if src:
                            images.append(src)
                else:
                    elements = await page.query_selector_all(selector)
                    for el in elements[:5]:
                        src = await el.get_attribute("src")
                        if src and 'logo' not in src.lower() and 'icon' not in src.lower():
                            images.append(src)
            except:
                continue
        
        return images
    
    async def _extract_keywords(self, page, content: str) -> List[str]:
        """Extract keywords/tags from page"""
        keywords = []
        
        # Try meta keywords
        try:
            meta = await page.query_selector('meta[name="keywords"]')
            if meta:
                kw_str = await meta.get_attribute("content")
                if kw_str:
                    keywords.extend([k.strip() for k in kw_str.split(',')[:5]])
        except:
            pass
        
        # Try article tags
        try:
            tags = await page.query_selector_all('.tag, .tags a, [rel="tag"]')
            for tag in tags[:5]:
                text = await tag.inner_text()
                if text and len(text) < 30:
                    keywords.append(text.strip())
        except:
            pass
        
        # Extract hashtags from content
        hashtags = re.findall(r'#(\w+)', content)
        keywords.extend(hashtags[:5])
        
        return list(set(keywords))[:10]
    
    async def scrape_user(self, user_id: str) -> Optional[ScrapedUser]:
        """
        Generic web doesn't have user profiles.
        Returns site info instead.
        """
        url = user_id if user_id.startswith('http') else f"https://{user_id}"
        domain = urlparse(url).netloc
        
        return ScrapedUser(
            platform=self.PLATFORM,
            user_id=domain,
            username=domain,
            display_name=domain,
            bio="",
            profile_url=url,
            followers=0,
            following=0,
        )
    
    async def scrape_trending(self) -> List[ScrapedTrend]:
        """Generic web doesn't have trending."""
        return []
