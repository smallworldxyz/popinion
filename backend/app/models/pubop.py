"""
pubop Data Models
Models for scraped social media data used in Real-World Simulation Prediction
"""

import json
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
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'ScrapedPost':
        """Create from dictionary"""
        timestamp = data.get("timestamp")
        if timestamp and isinstance(timestamp, str):
            timestamp = datetime.fromisoformat(timestamp)
        return cls(
            platform=data["platform"],
            post_id=data["post_id"],
            content=data.get("content", ""),
            author_id=data["author_id"],
            author_name=data.get("author_name", ""),
            timestamp=timestamp or datetime.now(),
            url=data.get("url"),
            likes=data.get("likes", 0),
            shares=data.get("shares", 0),
            comments=data.get("comments", 0),
            views=data.get("views", 0),
            media_urls=data.get("media_urls", []),
            hashtags=data.get("hashtags", []),
            mentions=data.get("mentions", []),
        )


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
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'ScrapedUser':
        """Create from dictionary"""
        created_at = data.get("created_at")
        if created_at and isinstance(created_at, str):
            created_at = datetime.fromisoformat(created_at)
        return cls(
            platform=data["platform"],
            user_id=data["user_id"],
            username=data.get("username", ""),
            display_name=data.get("display_name", ""),
            bio=data.get("bio", ""),
            profile_url=data.get("profile_url"),
            avatar_url=data.get("avatar_url"),
            followers=data.get("followers", 0),
            following=data.get("following", 0),
            post_count=data.get("post_count", 0),
            verified=data.get("verified", False),
            created_at=created_at,
        )


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
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'ScrapedTrend':
        """Create from dictionary"""
        timestamp = data.get("timestamp")
        if timestamp and isinstance(timestamp, str):
            timestamp = datetime.fromisoformat(timestamp)
        return cls(
            platform=data["platform"],
            topic=data["topic"],
            post_count=data.get("post_count", 0),
            timestamp=timestamp or datetime.now(),
            url=data.get("url"),
            category=data.get("category"),
        )


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
    
    def to_json(self, indent: int = 2) -> str:
        """Serialize to JSON string"""
        return json.dumps(self.to_dict(), ensure_ascii=False, indent=indent)
    
    def save(self, filepath: str) -> None:
        """Save to JSON file"""
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(self.to_json())
    
    @classmethod
    def load(cls, filepath: str) -> 'CrawlResult':
        """Load from JSON file"""
        with open(filepath, 'r', encoding='utf-8') as f:
            data = json.load(f)
        return cls.from_dict(data)
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'CrawlResult':
        """Create from dictionary"""
        crawled_at = data.get("crawled_at")
        if crawled_at and isinstance(crawled_at, str):
            crawled_at = datetime.fromisoformat(crawled_at)
        
        return cls(
            platform=data["platform"],
            query=data.get("query"),
            posts=[ScrapedPost.from_dict(p) for p in data.get("posts", [])],
            users=[ScrapedUser.from_dict(u) for u in data.get("users", [])],
            trends=[ScrapedTrend.from_dict(t) for t in data.get("trends", [])],
            crawled_at=crawled_at or datetime.now(),
            success=data.get("success", True),
            error=data.get("error"),
        )

