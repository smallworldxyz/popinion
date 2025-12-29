"""
pubop Real-World Data Scraper (Smart Edition)
Scrapes real news, posts, and multimedia about entities during simulation preparation.

Features:
1. Dynamic Source Discovery: Search Google/DuckDuckGo based on prompt context
2. Multimodal Support: Search for text, images, videos, reactions
3. AI/Fake Detection: Analyze content for AI markers
"""

import asyncio
import re
from typing import List, Optional, Dict, Any
from datetime import datetime
from urllib.parse import quote_plus

from ..models.pubop import ScrapedPost, CrawlResult
from ..utils.logger import get_logger
from .crawler import LightPandaClient, GenericWebCrawler
from .pubop_bridge import PubopBridge, RealDataSeed

logger = get_logger('pubop.realworld_scraper')


class RealWorldScraper:
    """
    Smart Scraper for real-world news and social media.
    
    Dynamically discovers sources based on simulation prompts.
    Analyzes content for authenticity.
    """
    
    def __init__(
        self,
        engine: str = "browserless",
        max_posts_per_entity: int = 5,
        timeout: int = 30000
    ):
        self.engine = engine
        self.max_posts_per_entity = max_posts_per_entity
        self.timeout = timeout
    
    async def scrape_entities(
        self,
        entities: List[Dict[str, Any]],
        simulation_requirement: str = "",
        max_total_posts: int = 50,
        progress_callback: Optional[callable] = None
    ) -> RealDataSeed:
        """
        Smart scrape entities based on context.
        
        Args:
            entities: List of entities
            simulation_requirement: User prompt to guide search intent
            max_total_posts: Max posts total
            progress_callback: Progress callback
        """
        all_posts: List[ScrapedPost] = []
        trending_topics: List[str] = []
        
        # 1. Analyze prompt to determine search intent
        intent = self._analyze_search_intent(simulation_requirement)
        logger.info(f"Search intent derived from prompt: {intent}")
        
        entity_names = [e.get("name", str(e)) if isinstance(e, dict) else str(e) for e in entities]
        total_entities = len(entity_names)
        
        try:
            async with LightPandaClient(engine=self.engine, timeout=self.timeout) as client:
                crawler = GenericWebCrawler(client)
                
                for i, entity_name in enumerate(entity_names):
                    if len(all_posts) >= max_total_posts:
                        break
                    
                    if progress_callback:
                        progress_callback(
                            i + 1, total_entities,
                            f"Smart searching: {entity_name} ({', '.join(intent['keywords'])})"
                        )
                    
                    # 2. Dynamic Search Discovery
                    # Instead of hardcoded sources, we search for the entity + intent
                    posts = await self._smart_search_entity(
                        client, crawler, entity_name, intent,
                        limit=self.max_posts_per_entity
                    )
                    
                    if posts:
                        # 3. Analyze content (Fake/AI detection)
                        analyzed_posts = self._analyze_posts_content(posts)
                        
                        all_posts.extend(analyzed_posts)
                        logger.info(f"Found {len(analyzed_posts)} items for {entity_name}")
                        
                        for post in analyzed_posts:
                            trending_topics.extend(post.hashtags)
                    else:
                        logger.warning(f"No results found for {entity_name}")
                    
                    await asyncio.sleep(1)
        
        except Exception as e:
            logger.error(f"Error during smart scraping: {e}")
        
        # Convert to initial posts
        initial_posts = []
        for post in all_posts:
            initial_posts.append({
                "user_id": 0,
                "username": post.author_name or "scraped_source",
                "content": post.content[:2000],
                "created_at": post.timestamp.isoformat() if post.timestamp else datetime.now().isoformat(),
                "likes": post.likes,
                "shares": post.shares,
                "comments": post.comments,
                "platform": post.platform,
                "source_url": post.url,
                "hashtags": post.hashtags,
                "is_real_world": True,
                # New fields for frontend visualization
                "content_analysis": getattr(post, "content_analysis", {}),
                "media_type": getattr(post, "media_type", "text"),
            })
        
        return RealDataSeed(
            platform="web",
            query=f"Smart Search: {simulation_requirement[:30]}...",
            crawled_at=datetime.now(),
            profiles=[],
            initial_posts=initial_posts,
            trending_topics=list(set(trending_topics))[:20],
            original_post_count=len(all_posts),
            original_user_count=0,
        )

    def _analyze_search_intent(self, prompt: str) -> Dict[str, Any]:
        """Analyze simulation prompt to guide search"""
        prompt_lower = prompt.lower()
        keywords = []
        
        # Detect focus topics
        if "opinion" in prompt_lower or "react" in prompt_lower:
            keywords.append("opinion")
            keywords.append("reaction")
        if "news" in prompt_lower or "article" in prompt_lower:
            keywords.append("news")
        if "video" in prompt_lower:
            keywords.append("video")
        if "scandal" in prompt_lower or "issue" in prompt_lower:
            keywords.append("scandal")
            keywords.append("controversy")
            
        # Default fallback
        if not keywords:
            keywords = ["news", "latest"]
            
        return {
            "keywords": keywords,
            "prefer_video": "video" in prompt_lower,
            "prefer_images": "image" in prompt_lower,
            "prefer_comments": "comment" in prompt_lower
        }

    async def _smart_search_entity(
        self,
        client: LightPandaClient,
        crawler: GenericWebCrawler,
        entity_name: str,
        intent: Dict[str, Any],
        limit: int = 5
    ) -> List[ScrapedPost]:
        """Performs search on DuckDuckGo/Google to find relevant URLs"""
        posts = []
        
        # Construct dynamic queries
        search_terms = [entity_name]
        search_terms.extend(intent["keywords"])
        query = " ".join(search_terms)
        
        # 1. Search DuckDuckGo (HTML version is easier to scrape)
        search_url = f"https://html.duckduckgo.com/html/?q={quote_plus(query)}"
        
        try:
            logger.info(f"Searching: {query}")
            # Use crawler's page interaction to get search results
            page = await client.get_page()
            await page.goto(search_url)
            await asyncio.sleep(2)
            
            # Extract result links
            links = []
            link_elements = await page.query_selector_all('.result__a')
            for el in link_elements[:limit]:
                href = await el.get_attribute('href')
                if href and 'duckduckgo' not in href:
                    links.append(href)
            
            logger.info(f"Discovered {len(links)} sources for {entity_name}")
            
            # 2. Scrape discovered URLs
            for url in links:
                if len(posts) >= limit:
                    break
                try:
                    post = await crawler.scrape_url(url)
                    if post:
                        # Detect media type
                        if intent["prefer_video"]:
                            post.media_type = "video" if ("youtube" in url or "video" in url) else "text"
                        else:
                            post.media_type = "text"
                        posts.append(post)
                except Exception as e:
                    logger.debug(f"Failed to scrape found link {url}: {e}")
                    
        except Exception as e:
            logger.warning(f"Search failed for {entity_name}: {e}")
        
        return posts

    def _analyze_posts_content(self, posts: List[ScrapedPost]) -> List[ScrapedPost]:
        """Analyze posts for AI generation and content quality"""
        for post in posts:
            analysis = {
                "is_likely_ai": False,
                "confidence": 0.0,
                "reason": []
            }
            
            content = (post.content or "").lower()
            
            # Simple heuristic detection (Placeholders for advanced model)
            ai_phrases = [
                "as an ai language model",
                "i cannot predict",
                "my knowledge cutoff",
                "in summary",
                "it appears that"
            ]
            
            matches = [p for p in ai_phrases if p in content]
            if matches:
                analysis["is_likely_ai"] = True
                analysis["confidence"] = 0.8 + (len(matches) * 0.05)
                analysis["reason"].append(f"Contains AI-like phrases: {matches}")
            
            # Repetitive structure check
            # (Simplified)
            
            # Metadata analysis
            if not post.author_name:
                analysis["reason"].append("Missing author attribution")
            
            post.content_analysis = analysis
            
        return posts
    
    async def scrape_entity_names(
        self,
        entity_names: List[str],
        simulation_requirement: str = "",
        max_total_posts: int = 50,
        progress_callback: Optional[callable] = None
    ) -> RealDataSeed:
        """Call with list of names"""
        entities = [{"name": name} for name in entity_names]
        return await self.scrape_entities(
            entities, simulation_requirement, max_total_posts, progress_callback
        )


def sync_scrape_entities(
    entity_names: List[str],
    simulation_requirement: str = "",
    max_posts: int = 50
) -> RealDataSeed:
    """Sync wrapper"""
    scraper = RealWorldScraper()
    loop = asyncio.new_event_loop()
    try:
        return loop.run_until_complete(
            scraper.scrape_entity_names(entity_names, simulation_requirement, max_posts)
        )
    finally:
        loop.close()
