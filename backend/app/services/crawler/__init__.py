"""
pubop Crawler Module
Web scraping infrastructure for Real-World Simulation Prediction

Provides:
- LightPandaClient: CDP connection to headless browser
- ProxyConfig: Proxy configuration with rotation pool
- BaseCrawler: Abstract base class for platform crawlers
- Platform crawlers: Telegram, Twitter, Facebook, Instagram, TikTok, YouTube, LINE, Zalo
- GenericWebCrawler: Scrapes any URL (news sites, blogs, articles)
"""

from .client import LightPandaClient, ProxyConfig, get_lightpanda_client
from .base import BaseCrawler
from .telegram import TelegramCrawler
from .twitter import TwitterCrawler
from .facebook import FacebookCrawler
from .instagram import InstagramCrawler
from .tiktok import TikTokCrawler
from .youtube import YouTubeCrawler
from .line import LINECrawler
from .zalo import ZaloCrawler
from .web import GenericWebCrawler

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
    "LINECrawler",
    "ZaloCrawler",
    # Generic
    "GenericWebCrawler",
]






