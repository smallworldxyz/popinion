"""
Tests for crawler module
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from app.services.crawler.client import LightPandaClient
from app.services.crawler.base import BaseCrawler
from app.models.pubop import ScrapedPost, ScrapedUser


class TestLightPandaClient:
    """Tests for LightPandaClient"""
    
    def test_init_default_endpoint(self):
        """Test default endpoint"""
        client = LightPandaClient()
        assert client.endpoint == "http://localhost:9222"
        assert client.is_connected == False
    
    def test_init_custom_endpoint(self):
        """Test custom endpoint"""
        client = LightPandaClient(endpoint="http://custom:9999")
        assert client.endpoint == "http://custom:9999"
    
    @pytest.mark.asyncio
    async def test_connect_not_connected_raises(self):
        """Test new_page raises when not connected"""
        client = LightPandaClient()
        with pytest.raises(ConnectionError):
            await client.new_page()


class TestBaseCrawler:
    """Tests for BaseCrawler abstract class"""
    
    def test_cannot_instantiate_abstract(self):
        """Test BaseCrawler cannot be instantiated directly"""
        mock_client = MagicMock(spec=LightPandaClient)
        with pytest.raises(TypeError):
            BaseCrawler(mock_client)
    
    def test_concrete_implementation(self):
        """Test concrete crawler implementation"""
        from datetime import datetime
        
        class TestCrawler(BaseCrawler):
            PLATFORM = "test"
            
            async def scrape_posts(self, query, limit=100):
                return [ScrapedPost(
                    platform=self.PLATFORM,
                    post_id="1",
                    content=f"Post about {query}",
                    author_id="author1",
                    author_name="Author",
                    timestamp=datetime.now()
                )]
            
            async def scrape_user(self, user_id):
                return ScrapedUser(
                    platform=self.PLATFORM,
                    user_id=user_id,
                    username=f"user_{user_id}",
                    display_name="Test User"
                )
            
            async def scrape_trending(self):
                return []
        
        mock_client = MagicMock(spec=LightPandaClient)
        crawler = TestCrawler(mock_client)
        assert crawler.PLATFORM == "test"
