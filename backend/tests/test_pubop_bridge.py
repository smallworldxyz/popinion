"""
Tests for pubop bridge and integration
"""

import pytest
from datetime import datetime
from unittest.mock import MagicMock

from app.models.pubop import ScrapedPost, ScrapedUser, CrawlResult, Platform
from app.services.pubop_bridge import PubopBridge, RealDataSeed


class TestRealDataSeed:
    """Tests for RealDataSeed dataclass"""
    
    def test_create_empty_seed(self):
        """Test creating empty seed"""
        seed = RealDataSeed(platform="telegram")
        assert seed.platform == "telegram"
        assert len(seed.profiles) == 0
        assert len(seed.initial_posts) == 0
    
    def test_seed_to_dict(self):
        """Test converting seed to dictionary"""
        seed = RealDataSeed(
            platform="twitter",
            query="test query",
            trending_topics=["topic1", "topic2"]
        )
        d = seed.to_dict()
        assert d["platform"] == "twitter"
        assert d["query"] == "test query"
        assert len(d["trending_topics"]) == 2


class TestPubopBridge:
    """Tests for PubopBridge"""
    
    def test_init_default(self):
        """Test default initialization"""
        bridge = PubopBridge()
        assert bridge.anonymize == True
        assert bridge.infer_personas == True
    
    def test_init_custom(self):
        """Test custom initialization"""
        bridge = PubopBridge(anonymize=False, infer_personas=False)
        assert bridge.anonymize == False
        assert bridge.infer_personas == False
    
    def test_users_to_profiles(self):
        """Test converting scraped users to profiles"""
        bridge = PubopBridge(anonymize=True)
        
        users = [
            ScrapedUser(
                platform="telegram",
                user_id="u1",
                username="testuser1",
                display_name="Test User 1",
                bio="Software developer",
                followers=1000
            ),
            ScrapedUser(
                platform="telegram",
                user_id="u2",
                username="testuser2",
                display_name="Test User 2",
                bio="Student at university",
                followers=500
            )
        ]
        
        profiles = bridge.users_to_profiles(users)
        
        assert len(profiles) == 2
        # Check anonymization
        assert profiles[0].user_name == "user_00000"
        assert profiles[1].user_name == "user_00001"
        # Check persona was generated
        assert len(profiles[0].persona) > 0
    
    def test_users_to_profiles_no_anonymize(self):
        """Test without anonymization"""
        bridge = PubopBridge(anonymize=False)
        
        users = [
            ScrapedUser(
                platform="twitter",
                user_id="123",
                username="realuser",
                display_name="Real User",
                bio="Bio text"
            )
        ]
        
        profiles = bridge.users_to_profiles(users)
        
        assert profiles[0].user_name == "realuser"
        assert profiles[0].name == "Real User"
    
    def test_posts_to_initial_posts(self):
        """Test converting posts to initial posts"""
        bridge = PubopBridge()
        
        # First create profiles
        users = [
            ScrapedUser(
                platform="twitter",
                user_id="author1",
                username="author",
                display_name="Author"
            )
        ]
        profiles = bridge.users_to_profiles(users)
        
        posts = [
            ScrapedPost(
                platform="twitter",
                post_id="p1",
                content="Hello world!",
                author_id="author1",
                author_name="author",
                timestamp=datetime.now(),
                likes=100
            )
        ]
        
        initial_posts = bridge.posts_to_initial_posts(posts, profiles)
        
        assert len(initial_posts) == 1
        assert initial_posts[0]["content"] == "Hello world!"
        assert initial_posts[0]["likes"] == 100
    
    def test_create_seed_from_crawl(self):
        """Test creating seed from crawl result"""
        bridge = PubopBridge()
        
        crawl_result = CrawlResult(
            platform="telegram",
            query="test",
            posts=[
                ScrapedPost(
                    platform="telegram",
                    post_id="1",
                    content="Post content",
                    author_id="a1",
                    author_name="Author",
                    timestamp=datetime.now()
                )
            ],
            users=[
                ScrapedUser(
                    platform="telegram",
                    user_id="a1",
                    username="author",
                    display_name="Author"
                )
            ]
        )
        
        seed = bridge.create_seed_from_crawl(crawl_result)
        
        assert seed.platform == "telegram"
        assert len(seed.profiles) == 1
        assert len(seed.initial_posts) == 1
    
    def test_infer_age(self):
        """Test age inference"""
        bridge = PubopBridge()
        
        assert bridge._infer_age("22 years old developer") == 22
        assert bridge._infer_age("University student") in range(18, 26)
        assert bridge._infer_age("No age info here") is None
    
    def test_infer_profession(self):
        """Test profession inference"""
        bridge = PubopBridge()
        
        assert bridge._infer_profession("Software developer at Google") == "developer"
        assert bridge._infer_profession("University student studying CS") == "student"
        assert bridge._infer_profession("Random text") is None
    
    def test_extract_topics(self):
        """Test topic extraction"""
        bridge = PubopBridge()
        
        topics = bridge._extract_topics("Tech enthusiast, love gaming and music")
        assert "technology" in topics
        assert "gaming" in topics or "entertainment" in topics
    
    def test_sanitize_content(self):
        """Test content sanitization"""
        bridge = PubopBridge()
        
        content = "Line 1\n\nLine 2   with   spaces"
        sanitized = bridge._sanitize_content(content)
        assert "\n\n" not in sanitized
        assert "   " not in sanitized
