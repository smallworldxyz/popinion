"""
pubop Crawler Module
Web scraping infrastructure for Real-World Simulation Prediction

Provides:
- LightPandaClient: CDP connection to headless browser (Browserless Chrome)
- ProxyConfig: Proxy configuration with rotation pool
- BaseCrawler: Abstract base class for platform crawlers
- TelegramCrawler: Scrapes public Telegram channels
- TwitterCrawler: Scrapes Twitter/X search and profiles
- FacebookCrawler: Scrapes public Facebook pages
- InstagramCrawler: Scrapes public Instagram profiles
- TikTokCrawler: Scrapes public TikTok profiles and hashtags

Example:
    from app.services.crawler import LightPandaClient, TelegramCrawler
    
    async with LightPandaClient() as client:
        crawler = TelegramCrawler(client)
        posts = await crawler.scrape_channel("dulorov", limit=100)
        
        for post in posts:
            print(f"{post.author_name}: {post.content[:50]}...")
    
    # With proxy rotation
    from app.services.crawler import ProxyConfig
    
    proxy = ProxyConfig.from_env()  # or ProxyConfig(server="http://proxy:8080")
    async with LightPandaClient(proxy=proxy) as client:
        ...
"""

from .client import LightPandaClient, ProxyConfig, get_lightpanda_client
from .base import BaseCrawler
from .telegram import TelegramCrawler
from .twitter import TwitterCrawler
from .facebook import FacebookCrawler
from .instagram import InstagramCrawler
from .tiktok import TikTokCrawler

__all__ = [
    # Core
    "LightPandaClient",
    "ProxyConfig",
    "get_lightpanda_client",
    "BaseCrawler",
    # Platform crawlers
    "TelegramCrawler",
    "TwitterCrawler", 
    "FacebookCrawler",
    "InstagramCrawler",
    "TikTokCrawler",
]



