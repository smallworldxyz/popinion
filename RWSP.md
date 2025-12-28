# pubop: Real-World Simulation Prediction

## 🎯 Mission

```
Real Data  →  Simulation  →  Prediction
```

---

## Architecture

```
LightPanda (CDP) → Crawlers → pubop_bridge → Simulation → Reports
```

---

## File Structure

```
backend/app/
├── models/
│   └── pubop.py                    # Data models
├── services/
│   ├── crawler/
│   │   ├── __init__.py
│   │   ├── client.py               # LightPanda CDP client
│   │   ├── base.py                 # Abstract BaseCrawler
│   │   ├── telegram.py
│   │   ├── twitter.py
│   │   └── facebook.py
│   ├── pubop_bridge.py             # Scraped → Simulation bridge
│   ├── oasis_profile_generator.py  # MOD: Add generate_profiles_from_scraped_users()
│   └── simulation_config_generator.py  # MOD: Add initial_posts_from_real
```

---

## Implementation Details

### 1. Data Models (`backend/app/models/pubop.py`)

```python
from dataclasses import dataclass, field
from typing import List, Optional
from datetime import datetime

@dataclass
class ScrapedPost:
    """A post scraped from social media"""
    platform: str           # telegram, twitter, facebook
    post_id: str
    content: str
    author_id: str
    author_name: str
    timestamp: datetime
    likes: int = 0
    shares: int = 0
    comments: int = 0
    media_urls: List[str] = field(default_factory=list)

@dataclass  
class ScrapedUser:
    """A user profile scraped from social media"""
    platform: str
    user_id: str
    username: str
    display_name: str
    bio: str = ""
    followers: int = 0
    following: int = 0
    post_count: int = 0

@dataclass
class ScrapedTrend:
    """A trending topic"""
    platform: str
    topic: str
    post_count: int
    timestamp: datetime
```

### 2. CDP Client (`backend/app/services/crawler/client.py`)

```python
from playwright.async_api import async_playwright

class LightPandaClient:
    """Chrome DevTools Protocol client for LightPanda"""
    
    def __init__(self, endpoint: str = "http://localhost:9222"):
        self.endpoint = endpoint
        self.browser = None
    
    async def connect(self):
        """Connect to LightPanda via CDP"""
        self.playwright = await async_playwright().start()
        self.browser = await self.playwright.chromium.connect_over_cdp(self.endpoint)
    
    async def new_page(self):
        context = await self.browser.new_context()
        return await context.new_page()
    
    async def close(self):
        if self.browser:
            await self.browser.close()
        if self.playwright:
            await self.playwright.stop()
```

### 3. Base Crawler (`backend/app/services/crawler/base.py`)

```python
from abc import ABC, abstractmethod
from typing import List
from ...models.pubop import ScrapedPost, ScrapedUser

class BaseCrawler(ABC):
    """Abstract base class for platform crawlers"""
    
    def __init__(self, client: LightPandaClient):
        self.client = client
    
    @abstractmethod
    async def scrape_posts(self, query: str, limit: int = 100) -> List[ScrapedPost]:
        """Scrape posts matching query"""
        pass
    
    @abstractmethod
    async def scrape_user(self, user_id: str) -> ScrapedUser:
        """Scrape user profile"""
        pass
    
    @abstractmethod
    async def scrape_trending(self) -> List[str]:
        """Get trending topics"""
        pass
```

### 4. Bridge Service (`backend/app/services/pubop_bridge.py`)

```python
from typing import List
from ..models.pubop import ScrapedPost, ScrapedUser
from .oasis_profile_generator import OasisAgentProfile

class PubopBridge:
    """Bridge scraped data to simulation"""
    
    def users_to_profiles(self, users: List[ScrapedUser]) -> List[OasisAgentProfile]:
        """Convert scraped users to OASIS agent profiles"""
        profiles = []
        for i, user in enumerate(users):
            profile = OasisAgentProfile(
                user_id=i,
                user_name=user.username,
                name=user.display_name,
                bio=user.bio,
                persona=f"Real user from {user.platform}: {user.bio}",
                follower_count=user.followers,
                friend_count=user.following,
                statuses_count=user.post_count
            )
            profiles.append(profile)
        return profiles
    
    def posts_to_initial_posts(self, posts: List[ScrapedPost]) -> List[dict]:
        """Convert scraped posts to simulation initial posts"""
        return [
            {
                "content": post.content,
                "author_id": post.author_id,
                "likes": post.likes,
                "timestamp": post.timestamp.isoformat()
            }
            for post in posts
        ]
```

---

## Docker Setup

```yaml
# Add to docker-compose.yml
services:
  lightpanda:
    image: ghcr.io/nicxleo/nicecmd:latest  # LightPanda image
    container_name: pubop-lightpanda
    ports:
      - "9222:9222"
    command: ["--remote-debugging-port=9222", "--remote-debugging-address=0.0.0.0"]
    restart: unless-stopped
```

> **Note**: Verify latest LightPanda Docker image at https://github.com/nicxleo/nicecmd

---

## API Requirements

| Platform | API/Method | Credentials |
|----------|------------|-------------|
| Telegram | Bot API + Web scraping | Bot token (optional) |
| X/Twitter | Web scraping only | None (no API needed) |
| Facebook | Graph API | App ID + Secret (for public pages) |

---

## Tasks

### Phase 1: Infrastructure [P0] - 3.5h ✅
- [x] Add LightPanda to docker-compose.yml
- [x] Create `backend/app/models/pubop.py`
- [x] Create `backend/app/services/crawler/__init__.py`
- [x] Create `backend/app/services/crawler/client.py`
- [x] Create `backend/app/services/crawler/base.py`

### Phase 2: Crawlers [P0] - 11h ✅
- [x] Create `telegram.py` crawler
- [x] Create `twitter.py` crawler  
- [x] Create `facebook.py` crawler

### Phase 3: Integration [P1] - 6h ✅
- [x] Create `pubop_bridge.py`
- [x] Add `generate_profiles_from_scraped_users()` to `oasis_profile_generator.py`
- [x] Add `inject_real_data_seed()` to `simulation_config_generator.py`

### Phase 4: Prediction [P2] - 5h
- [ ] Add comparison tools to ReportAgent

---

**Total: ~25.5 hours**
