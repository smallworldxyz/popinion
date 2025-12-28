"""
Tests for pubop data models
"""

import pytest
from datetime import datetime
from app.models.pubop import (
    ScrapedPost, 
    ScrapedUser, 
    ScrapedTrend, 
    CrawlResult,
    Platform
)


class TestScrapedPost:
    """Tests for ScrapedPost dataclass"""
    
    def test_create_basic_post(self):
        """Test creating a basic post"""
        post = ScrapedPost(
            platform="twitter",
            post_id="123",
            content="Hello world",
            author_id="user1",
            author_name="Test User",
            timestamp=datetime.now()
        )
        assert post.platform == "twitter"
        assert post.content == "Hello world"
        assert post.likes == 0  # Default value
    
    def test_post_to_dict(self):
        """Test converting post to dictionary"""
        now = datetime.now()
        post = ScrapedPost(
            platform="telegram",
            post_id="456",
            content="Test content",
            author_id="user2",
            author_name="Author",
            timestamp=now,
            likes=100,
            shares=50
        )
        d = post.to_dict()
        assert d["platform"] == "telegram"
        assert d["likes"] == 100
        assert d["shares"] == 50
        assert "timestamp" in d


class TestScrapedUser:
    """Tests for ScrapedUser dataclass"""
    
    def test_create_user(self):
        """Test creating a user"""
        user = ScrapedUser(
            platform="facebook",
            user_id="u123",
            username="testuser",
            display_name="Test User",
            bio="Hello, I'm a test user",
            followers=1000
        )
        assert user.username == "testuser"
        assert user.followers == 1000
        assert user.following == 0  # Default
    
    def test_user_to_dict(self):
        """Test converting user to dictionary"""
        user = ScrapedUser(
            platform="twitter",
            user_id="u456",
            username="another",
            display_name="Another User"
        )
        d = user.to_dict()
        assert d["platform"] == "twitter"
        assert d["username"] == "another"


class TestScrapedTrend:
    """Tests for ScrapedTrend dataclass"""
    
    def test_create_trend(self):
        """Test creating a trend"""
        trend = ScrapedTrend(
            platform="twitter",
            topic="#Python",
            post_count=50000,
            timestamp=datetime.now()
        )
        assert trend.topic == "#Python"
        assert trend.post_count == 50000


class TestCrawlResult:
    """Tests for CrawlResult dataclass"""
    
    def test_empty_result(self):
        """Test creating empty result"""
        result = CrawlResult(platform="telegram")
        assert result.success == True
        assert len(result.posts) == 0
        assert len(result.users) == 0
    
    def test_result_with_data(self):
        """Test result with posts"""
        post = ScrapedPost(
            platform="telegram",
            post_id="1",
            content="Test",
            author_id="a1",
            author_name="Author",
            timestamp=datetime.now()
        )
        result = CrawlResult(
            platform="telegram",
            query="test",
            posts=[post]
        )
        assert len(result.posts) == 1
        d = result.to_dict()
        assert len(d["posts"]) == 1


class TestPlatformEnum:
    """Tests for Platform enum"""
    
    def test_platform_values(self):
        """Test platform enum values"""
        assert Platform.TELEGRAM.value == "telegram"
        assert Platform.TWITTER.value == "twitter"
        assert Platform.FACEBOOK.value == "facebook"
