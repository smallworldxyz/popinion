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
- YouTubeCrawler: Scrapes public YouTube channels and videos
"""

from .client import LightPandaClient, ProxyConfig, get_lightpanda_client
from .base import BaseCrawler
from .telegram import TelegramCrawler
from .twitter import TwitterCrawler
from .facebook import FacebookCrawler
from .instagram import InstagramCrawler
from .tiktok import TikTokCrawler
from .youtube import YouTubeCrawler

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
    "YouTubeCrawler",
]




