"""
pubop Crawler Module
Web scraping infrastructure for Real-World Simulation Prediction

Provides:
- LightPandaClient: CDP connection to LightPanda headless browser
- BaseCrawler: Abstract base class for platform crawlers
- TelegramCrawler: Scrapes public Telegram channels
- TwitterCrawler: Scrapes Twitter/X search and profiles
- FacebookCrawler: Scrapes public Facebook pages

Example:
    from app.services.crawler import LightPandaClient, TelegramCrawler
    
    async with LightPandaClient() as client:
        crawler = TelegramCrawler(client)
        posts = await crawler.scrape_channel("dulorov", limit=100)
        
        for post in posts:
            print(f"{post.author_name}: {post.content[:50]}...")
"""

from .client import LightPandaClient, get_lightpanda_client
from .base import BaseCrawler
from .telegram import TelegramCrawler
from .twitter import TwitterCrawler
from .facebook import FacebookCrawler

__all__ = [
    # Core
    "LightPandaClient",
    "get_lightpanda_client",
    "BaseCrawler",
    # Platform crawlers
    "TelegramCrawler",
    "TwitterCrawler", 
    "FacebookCrawler",
]
