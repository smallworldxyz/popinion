"""
pubop Data Models
Models for scraped social media data used in Real-World Simulation Prediction
"""

from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any
from datetime import datetime
from enum import Enum


class Platform(str, Enum):
    """Supported social media platforms"""
    TELEGRAM = "telegram"
    TWITTER = "twitter"
    FACEBOOK = "facebook"
    YOUTUBE = "youtube"
    REDDIT = "reddit"


@dataclass
class ScrapedPost:
    """A post scraped from social media"""
    platform: str
    post_id: str
    content: str
    author_id: str
    author_name: str
    timestamp: datetime
    url: Optional[str] = None
    likes: int = 0
    shares: int = 0
    comments: int = 0
    views: int = 0
    media_urls: List[str] = field(default_factory=list)
    hashtags: List[str] = field(default_factory=list)
    mentions: List[str] = field(default_factory=list)
    raw_data: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary"""
        return {
            "platform": self.platform,
            "post_id": self.post_id,
            "content": self.content,
            "author_id": self.author_id,
            "author_name": self.author_name,
            "timestamp": self.timestamp.isoformat() if self.timestamp else None,
            "url": self.url,
            "likes": self.likes,
            "shares": self.shares,
            "comments": self.comments,
            "views": self.views,
            "media_urls": self.media_urls,
            "hashtags": self.hashtags,
            "mentions": self.mentions,
        }


@dataclass
class ScrapedUser:
    """A user profile scraped from social media"""
    platform: str
    user_id: str
    username: str
    display_name: str
    bio: str = ""
    profile_url: Optional[str] = None
    avatar_url: Optional[str] = None
    followers: int = 0
    following: int = 0
    post_count: int = 0
    verified: bool = False
    created_at: Optional[datetime] = None
    raw_data: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary"""
        return {
            "platform": self.platform,
            "user_id": self.user_id,
            "username": self.username,
            "display_name": self.display_name,
            "bio": self.bio,
            "profile_url": self.profile_url,
            "avatar_url": self.avatar_url,
            "followers": self.followers,
            "following": self.following,
            "post_count": self.post_count,
            "verified": self.verified,
            "created_at": self.created_at.isoformat() if self.created_at else None,
        }


@dataclass
class ScrapedTrend:
    """A trending topic from social media"""
    platform: str
    topic: str
    post_count: int
    timestamp: datetime
    url: Optional[str] = None
    category: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary"""
        return {
            "platform": self.platform,
            "topic": self.topic,
            "post_count": self.post_count,
            "timestamp": self.timestamp.isoformat() if self.timestamp else None,
            "url": self.url,
            "category": self.category,
        }


@dataclass
class CrawlResult:
    """Result from a crawl operation"""
    platform: str
    query: Optional[str] = None
    posts: List[ScrapedPost] = field(default_factory=list)
    users: List[ScrapedUser] = field(default_factory=list)
    trends: List[ScrapedTrend] = field(default_factory=list)
    crawled_at: datetime = field(default_factory=datetime.now)
    success: bool = True
    error: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary"""
        return {
            "platform": self.platform,
            "query": self.query,
            "posts": [p.to_dict() for p in self.posts],
            "users": [u.to_dict() for u in self.users],
            "trends": [t.to_dict() for t in self.trends],
            "crawled_at": self.crawled_at.isoformat(),
            "success": self.success,
            "error": self.error,
        }
