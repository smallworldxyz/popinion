"""
pubop Bridge Service
Bridges scraped real-world data to OASIS simulation engine

This module provides the connection between:
- Real social media data (from crawlers)
- OASIS simulation profiles and initial posts

Key functions:
- Convert scraped users → OASIS agent profiles
- Convert scraped posts → simulation initial posts
- Generate behavioral parameters from real data patterns
"""

import random
from typing import List, Optional, Dict, Any, Callable
from datetime import datetime
from dataclasses import dataclass, field

from ..models.pubop import ScrapedPost, ScrapedUser, CrawlResult
from ..utils.logger import get_logger
from .oasis_profile_generator import OasisAgentProfile

logger = get_logger('pubop.bridge')


# MBTI types for random assignment when not inferrable
MBTI_TYPES = [
    "INTJ", "INTP", "ENTJ", "ENTP",
    "INFJ", "INFP", "ENFJ", "ENFP", 
    "ISTJ", "ISFJ", "ESTJ", "ESFJ",
    "ISTP", "ISFP", "ESTP", "ESFP"
]


@dataclass
class RealDataSeed:
    """
    Container for real-world data to seed into simulation.
    
    This is passed to SimulationConfigGenerator to inject
    real content into the simulation.
    """
    # Source information
    platform: str
    query: Optional[str] = None
    crawled_at: datetime = field(default_factory=datetime.now)
    
    # Scraped content
    profiles: List[OasisAgentProfile] = field(default_factory=list)
    initial_posts: List[Dict[str, Any]] = field(default_factory=list)
    trending_topics: List[str] = field(default_factory=list)
    
    # Statistics
    original_post_count: int = 0
    original_user_count: int = 0
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for serialization"""
        return {
            "platform": self.platform,
            "query": self.query,
            "crawled_at": self.crawled_at.isoformat(),
            "profile_count": len(self.profiles),
            "initial_post_count": len(self.initial_posts),
            "trending_topics": self.trending_topics,
            "original_post_count": self.original_post_count,
            "original_user_count": self.original_user_count,
        }


class PubopBridge:
    """
    Bridge between real-world scraped data and OASIS simulation.
    
    Converts scraped social media content into formats compatible
    with the OASIS multi-agent simulation engine.
    
    Example:
        bridge = PubopBridge()
        
        # From crawl result
        seed = bridge.create_seed_from_crawl(crawl_result)
        
        # Or manually
        profiles = bridge.users_to_profiles(scraped_users)
        posts = bridge.posts_to_initial_posts(scraped_posts)
    """
    
    def __init__(
        self,
        anonymize: bool = True,
        infer_personas: bool = True
    ):
        """
        Initialize bridge.
        
        Args:
            anonymize: Whether to anonymize real user data
            infer_personas: Whether to infer persona traits from content
        """
        self.anonymize = anonymize
        self.infer_personas = infer_personas
    
    def create_seed_from_crawl(
        self,
        crawl_result: CrawlResult,
        max_profiles: int = 100,
        max_posts: int = 500
    ) -> RealDataSeed:
        """
        Create a complete seed from a crawl result.
        
        Args:
            crawl_result: Result from a crawler
            max_profiles: Maximum profiles to generate
            max_posts: Maximum posts to include
            
        Returns:
            RealDataSeed ready for simulation
        """
        logger.info(
            f"Creating seed from {crawl_result.platform} crawl: "
            f"{len(crawl_result.posts)} posts, {len(crawl_result.users)} users"
        )
        
        # Convert users to profiles
        profiles = self.users_to_profiles(
            crawl_result.users[:max_profiles]
        )
        
        # If we have more posts than users, extract users from posts
        if len(profiles) < len(crawl_result.posts[:max_posts]):
            additional_profiles = self._extract_profiles_from_posts(
                crawl_result.posts,
                existing_ids={p.source_entity_uuid for p in profiles},
                limit=max_profiles - len(profiles)
            )
            profiles.extend(additional_profiles)
        
        # Convert posts
        initial_posts = self.posts_to_initial_posts(
            crawl_result.posts[:max_posts],
            profiles
        )
        
        # Extract trending topics
        trending = [t.topic for t in crawl_result.trends[:20]]
        
        return RealDataSeed(
            platform=crawl_result.platform,
            query=crawl_result.query,
            crawled_at=crawl_result.crawled_at,
            profiles=profiles,
            initial_posts=initial_posts,
            trending_topics=trending,
            original_post_count=len(crawl_result.posts),
            original_user_count=len(crawl_result.users),
        )
    
    def users_to_profiles(
        self,
        users: List[ScrapedUser],
        start_id: int = 0
    ) -> List[OasisAgentProfile]:
        """
        Convert scraped users to OASIS agent profiles.
        
        Args:
            users: List of scraped user profiles
            start_id: Starting user_id for profiles
            
        Returns:
            List of OasisAgentProfile objects
        """
        profiles = []
        
        for i, user in enumerate(users):
            profile = self._user_to_profile(user, start_id + i)
            if profile:
                profiles.append(profile)
        
        logger.info(f"Converted {len(profiles)} users to profiles")
        return profiles
    
    def _user_to_profile(
        self,
        user: ScrapedUser,
        user_id: int
    ) -> OasisAgentProfile:
        """Convert a single user to profile"""
        
        # Anonymize username if enabled
        if self.anonymize:
            username = f"user_{user_id:05d}"
            display_name = f"User {user_id}"
        else:
            username = self._sanitize_username(user.username)
            display_name = user.display_name or username
        
        # Generate persona from bio
        persona = self._generate_persona_from_bio(user.bio, user.platform)
        
        # Infer traits from content
        age = self._infer_age(user.bio) if self.infer_personas else None
        profession = self._infer_profession(user.bio) if self.infer_personas else None
        topics = self._extract_topics(user.bio)
        
        return OasisAgentProfile(
            user_id=user_id,
            user_name=username,
            name=display_name,
            bio=user.bio[:500] if user.bio else "",
            persona=persona,
            follower_count=user.followers,
            friend_count=user.following,
            statuses_count=user.post_count,
            age=age,
            profession=profession,
            interested_topics=topics,
            mbti=random.choice(MBTI_TYPES),
            source_entity_uuid=user.user_id,
            source_entity_type=f"{user.platform}_user",
        )
    
    def _extract_profiles_from_posts(
        self,
        posts: List[ScrapedPost],
        existing_ids: set,
        limit: int
    ) -> List[OasisAgentProfile]:
        """Extract minimal profiles from post authors"""
        profiles = []
        seen_authors = existing_ids.copy()
        
        for post in posts:
            if post.author_id in seen_authors:
                continue
            
            if len(profiles) >= limit:
                break
            
            # Create minimal profile from post author
            profile = OasisAgentProfile(
                user_id=len(existing_ids) + len(profiles),
                user_name=self._sanitize_username(post.author_name) if not self.anonymize 
                          else f"user_{len(existing_ids) + len(profiles):05d}",
                name=post.author_name if not self.anonymize else f"User {len(existing_ids) + len(profiles)}",
                bio="",
                persona=f"Active {post.platform} user who posts about various topics.",
                source_entity_uuid=post.author_id,
                source_entity_type=f"{post.platform}_user",
                mbti=random.choice(MBTI_TYPES),
            )
            
            profiles.append(profile)
            seen_authors.add(post.author_id)
        
        return profiles
    
    def posts_to_initial_posts(
        self,
        posts: List[ScrapedPost],
        profiles: List[OasisAgentProfile]
    ) -> List[Dict[str, Any]]:
        """
        Convert scraped posts to simulation initial posts.
        
        Args:
            posts: List of scraped posts
            profiles: Available agent profiles (to map authors)
            
        Returns:
            List of post dictionaries for simulation
        """
        # Build author mapping
        author_to_profile = {}
        for profile in profiles:
            if profile.source_entity_uuid:
                author_to_profile[profile.source_entity_uuid] = profile
        
        initial_posts = []
        
        for post in posts:
            # Find matching profile
            profile = author_to_profile.get(post.author_id)
            if not profile:
                # Skip posts without matching profile
                continue
            
            # Convert to simulation format
            sim_post = {
                "user_id": profile.user_id,
                "username": profile.user_name,
                "content": self._sanitize_content(post.content),
                "created_at": post.timestamp.isoformat() if post.timestamp else datetime.now().isoformat(),
                "likes": post.likes,
                "shares": post.shares,
                "comments": post.comments,
                "platform": post.platform,
            }
            
            # Add hashtags if available
            if post.hashtags:
                sim_post["hashtags"] = post.hashtags
            
            initial_posts.append(sim_post)
        
        logger.info(f"Converted {len(initial_posts)} posts to initial posts")
        return initial_posts
    
    def _generate_persona_from_bio(self, bio: str, platform: str) -> str:
        """Generate a persona description from user bio"""
        if not bio:
            return f"An active {platform} user who engages with content on various topics."
        
        # Clean and truncate bio
        clean_bio = bio.strip()[:300]
        
        persona = f"A real {platform} user. "
        if clean_bio:
            persona += f"Profile description: {clean_bio}. "
        persona += "Behaves authentically based on their observed online patterns."
        
        return persona
    
    def _infer_age(self, bio: str) -> Optional[int]:
        """Attempt to infer age from bio content"""
        if not bio:
            return None
        
        bio_lower = bio.lower()
        
        # Check for explicit age mentions
        import re
        age_match = re.search(r'\b(\d{1,2})\s*(years? old|y\.?o\.?|yo)\b', bio_lower)
        if age_match:
            age = int(age_match.group(1))
            if 13 <= age <= 99:
                return age
        
        # Infer from keywords
        if any(word in bio_lower for word in ['student', 'university', 'college', 'grad']):
            return random.randint(18, 25)
        if any(word in bio_lower for word in ['dad', 'mom', 'father', 'mother', 'parent']):
            return random.randint(30, 45)
        if any(word in bio_lower for word in ['retired', 'grandpa', 'grandma']):
            return random.randint(60, 75)
        
        return None
    
    def _infer_profession(self, bio: str) -> Optional[str]:
        """Attempt to infer profession from bio"""
        if not bio:
            return None
        
        bio_lower = bio.lower()
        
        profession_keywords = {
            'developer': ['developer', 'programmer', 'coder', 'software', 'engineer'],
            'designer': ['designer', 'artist', 'creative', 'illustrator'],
            'writer': ['writer', 'author', 'journalist', 'editor', 'blogger'],
            'teacher': ['teacher', 'professor', 'educator', 'instructor'],
            'entrepreneur': ['entrepreneur', 'founder', 'ceo', 'startup'],
            'student': ['student', 'studying', 'university', 'college'],
            'healthcare': ['doctor', 'nurse', 'medical', 'healthcare'],
            'marketing': ['marketing', 'social media', 'content creator'],
        }
        
        for profession, keywords in profession_keywords.items():
            if any(kw in bio_lower for kw in keywords):
                return profession
        
        return None
    
    def _extract_topics(self, bio: str) -> List[str]:
        """Extract interested topics from bio"""
        if not bio:
            return []
        
        bio_lower = bio.lower()
        topics = []
        
        topic_keywords = {
            'technology': ['tech', 'technology', 'ai', 'machine learning', 'crypto', 'blockchain'],
            'sports': ['sports', 'football', 'basketball', 'soccer', 'fitness', 'gym'],
            'music': ['music', 'musician', 'singer', 'band', 'guitar', 'piano'],
            'gaming': ['gaming', 'gamer', 'esports', 'twitch', 'streamer'],
            'politics': ['politics', 'political', 'democracy', 'activist'],
            'business': ['business', 'finance', 'investing', 'stocks', 'trading'],
            'entertainment': ['movies', 'films', 'tv', 'netflix', 'entertainment'],
            'food': ['food', 'cooking', 'chef', 'foodie', 'restaurant'],
            'travel': ['travel', 'traveler', 'adventure', 'exploring'],
            'art': ['art', 'artist', 'photography', 'gallery'],
        }
        
        for topic, keywords in topic_keywords.items():
            if any(kw in bio_lower for kw in keywords):
                topics.append(topic)
        
        return topics[:5]  # Limit to 5 topics
    
    def _sanitize_username(self, username: str) -> str:
        """Sanitize username for simulation"""
        import re
        # Remove special characters, keep alphanumeric and underscore
        clean = re.sub(r'[^\w]', '_', username)
        # Remove consecutive underscores
        clean = re.sub(r'_+', '_', clean)
        # Strip leading/trailing underscores
        clean = clean.strip('_')
        return clean[:30] or "user"
    
    def _sanitize_content(self, content: str) -> str:
        """Sanitize post content for simulation"""
        if not content:
            return ""
        
        # Remove excessive whitespace
        import re
        clean = re.sub(r'\s+', ' ', content.strip())
        
        # Truncate to reasonable length
        return clean[:2000]
