"""
Tests for platform crawlers (Telegram, Twitter, Facebook)
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from datetime import datetime

from app.services.crawler import (
    TelegramCrawler,
    TwitterCrawler,
    FacebookCrawler,
    LightPandaClient,
)
from app.models.pubop import Platform


class TestTelegramCrawler:
    """Tests for TelegramCrawler"""
    
    def test_platform_identifier(self):
        """Test platform is set correctly"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TelegramCrawler(mock_client)
        assert crawler.PLATFORM == Platform.TELEGRAM.value
    
    def test_base_url(self):
        """Test base URL is correct"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TelegramCrawler(mock_client)
        assert "t.me" in crawler.BASE_URL
    
    def test_parse_count_thousands(self):
        """Test parsing K notation"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TelegramCrawler(mock_client)
        assert crawler._parse_count("1.5K") == 1500
        assert crawler._parse_count("100K") == 100000
    
    def test_parse_count_millions(self):
        """Test parsing M notation"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TelegramCrawler(mock_client)
        assert crawler._parse_count("2.5M") == 2500000
        assert crawler._parse_count("1M") == 1000000
    
    def test_parse_count_plain_number(self):
        """Test parsing plain numbers"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TelegramCrawler(mock_client)
        assert crawler._parse_count("1234") == 1234
        assert crawler._parse_count("0") == 0


class TestTwitterCrawler:
    """Tests for TwitterCrawler"""
    
    def test_platform_identifier(self):
        """Test platform is set correctly"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TwitterCrawler(mock_client)
        assert crawler.PLATFORM == Platform.TWITTER.value
    
    def test_base_url(self):
        """Test base URL is correct"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TwitterCrawler(mock_client)
        assert "x.com" in crawler.BASE_URL
    
    def test_search_url(self):
        """Test search URL is correct"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TwitterCrawler(mock_client)
        assert "search" in crawler.SEARCH_URL
    
    def test_parse_count(self):
        """Test count parsing"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TwitterCrawler(mock_client)
        assert crawler._parse_count("12.5K") == 12500
        assert crawler._parse_count("1,234") == 1234
        assert crawler._parse_count("3M") == 3000000


class TestFacebookCrawler:
    """Tests for FacebookCrawler"""
    
    def test_platform_identifier(self):
        """Test platform is set correctly"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = FacebookCrawler(mock_client)
        assert crawler.PLATFORM == Platform.FACEBOOK.value
    
    def test_base_url_mobile(self):
        """Test uses mobile URL for scraping"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = FacebookCrawler(mock_client)
        assert "m.facebook.com" in crawler.BASE_URL
    
    def test_desktop_url(self):
        """Test desktop URL for links"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = FacebookCrawler(mock_client)
        assert "www.facebook.com" in crawler.DESKTOP_URL
    
    def test_parse_count(self):
        """Test count parsing"""
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = FacebookCrawler(mock_client)
        assert crawler._parse_count("1.5K likes") == 1500
        assert crawler._parse_count("2M followers") == 2000000


class TestCrawlerConsistency:
    """Test consistency across all crawlers"""
    
    def test_all_crawlers_have_platform(self):
        """All crawlers should have PLATFORM defined"""
        mock_client = MagicMock(spec=LightPandaClient)
        
        crawlers = [
            TelegramCrawler(mock_client),
            TwitterCrawler(mock_client),
            FacebookCrawler(mock_client),
        ]
        
        for crawler in crawlers:
            assert hasattr(crawler, 'PLATFORM')
            assert crawler.PLATFORM in [p.value for p in Platform]
    
    def test_all_crawlers_have_required_methods(self):
        """All crawlers should implement required methods"""
        mock_client = MagicMock(spec=LightPandaClient)
        
        crawlers = [
            TelegramCrawler(mock_client),
            TwitterCrawler(mock_client),
            FacebookCrawler(mock_client),
        ]
        
        for crawler in crawlers:
            assert hasattr(crawler, 'scrape_posts')
            assert hasattr(crawler, 'scrape_user')
            assert hasattr(crawler, 'scrape_trending')
            assert hasattr(crawler, 'scrape_channel')
