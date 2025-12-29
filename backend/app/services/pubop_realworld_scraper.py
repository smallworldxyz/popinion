"""
pubop Real-World Data Scraper (Smart Edition)
Scrapes real news, posts, and multimedia about entities during simulation preparation.

Features:
1. Dynamic Source Discovery: Search Google/DuckDuckGo based on prompt context
2. Multimodal Support: Search for text, images, videos, reactions
3. AI/Fake Detection: Analyze content for AI markers
4. AI Search Planning: Uses LLM to generate targeted search queries
"""

import asyncio
import re
import json
from typing import List, Optional, Dict, Any
from datetime import datetime
from urllib.parse import quote_plus

from ..models.pubop import ScrapedPost, CrawlResult
from ..utils.logger import get_logger
from .crawler import LightPandaClient, GenericWebCrawler
from .pubop_bridge import PubopBridge, RealDataSeed
from ..config import Config

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
        
        # Initialize LLM client for smart planning
        self.llm_client = None
        try:
            from openai import OpenAI
            if Config.LLM_API_KEY:
                self.llm_client = OpenAI(
                    api_key=Config.LLM_API_KEY, 
                    base_url=Config.LLM_BASE_URL
                )
                self.llm_model = Config.LLM_MODEL
                logger.info(f"LLM Search Planner initialized: {self.llm_model}")
        except Exception as e:
            logger.warning(f"LLM client init failed: {e}")
    
    async def scrape_entities(
        self,
        entities: List[Dict[str, Any]],
        simulation_requirement: str = "",
        document_text: str = "",
        max_total_posts: int = 50,
        progress_callback: Optional[callable] = None
    ) -> RealDataSeed:
        """
        Smart scrape entities based on context.
        
        Args:
            entities: List of entities
            simulation_requirement: User prompt to guide search intent
            document_text: Reality seeds content
            max_total_posts: Max posts total
            progress_callback: Progress callback
        """
        all_posts: List[ScrapedPost] = []
        trending_topics: List[str] = []
        
        entity_names = [e.get("name", str(e)) if isinstance(e, dict) else str(e) for e in entities]
        total_entities = len(entity_names)
        
        # 1. Generate Smart Search Plan using LLM
        search_plan = {}
        if self.llm_client and simulation_requirement:
            if progress_callback:
                progress_callback(0, total_entities, "AI generating search plan...")
            search_plan = await self._generate_ai_search_plan(
                entity_names, simulation_requirement, document_text
            )
        
        # Fallback intent analysis
        fallback_intent = self._analyze_search_intent(simulation_requirement)
        
        try:
            async with LightPandaClient(engine=self.engine, timeout=self.timeout) as client:
                crawler = GenericWebCrawler(client)
                
                for i, entity_name in enumerate(entity_names):
                    if len(all_posts) >= max_total_posts:
                        break
                    
                    # Get queries for this entity from plan, or use fallback
                    entity_queries = search_plan.get(entity_name, [])
                    if not entity_queries:
                        # Fallback logic
                        keywords = fallback_intent["keywords"]
                        entity_queries = [f"{entity_name} {kw}" for kw in keywords[:2]]
                    
                    if progress_callback:
                        progress_callback(
                            i + 1, total_entities,
                            f"Searching: {entity_queries[0]}..."
                        )
                    
                    # 2. Execute Search Queries
                    entity_posts = []
                    for query in entity_queries[:2]: # Limit queries per entity
                        posts = await self._execute_search(
                            client, crawler, query, 
                            limit=max(2, self.max_posts_per_entity // 2)
                        )
                        entity_posts.extend(posts)
                    
                    if entity_posts:
                        # 3. Analyze content (Fake/AI detection)
                        analyzed_posts = self._analyze_posts_content(entity_posts)
                        
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
                "poster_agent_id": 99999,  # Use special ID for "World/System" content
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
            query=f"AI Plan: {simulation_requirement[:30]}...",
            crawled_at=datetime.now(),
            profiles=[],
            initial_posts=initial_posts,
            trending_topics=list(set(trending_topics))[:20],
            original_post_count=len(all_posts),
            original_user_count=0,
        )

    async def _generate_ai_search_plan(
        self, 
        entities: List[str], 
        requirement: str,
        doc_text: str
    ) -> Dict[str, List[str]]:
        """Generate specific search queries using LLM"""
        try:
            prompt = f"""
            You are a Search Specialist for a simulation engine.
            Goal: Generate search queries to find real-world data (news, opinions, facts) to ground a simulation.
            
            Simulation Prompt: "{requirement}"
            Background Doc: "{doc_text[:1000]}..."
            Entities: {', '.join(entities[:10])}
            
            Task: For each entity, generate 2 specific search queries that would yield relevant data for the simulation.
            Queries should be specific (e.g., instead of just "Bitcoin news", use "Bitcoin price crash reaction 2024").
            
            Output strictly valid JSON format:
            {{
                "Entity Name": ["Query 1", "Query 2"],
                ...
            }}
            """
            
            response = self.llm_client.chat.completions.create(
                model=self.llm_model,
                messages=[{"role": "user", "content": prompt}],
                response_format={"type": "json_object"}
            )
            
            content = response.choices[0].message.content
            plan = json.loads(content)
            logger.info(f"AI Search Plan Generated: {len(plan)} entities")
            return plan
            
        except Exception as e:
            logger.error(f"Failed to generate AI search plan: {e}")
            return {}

    def _analyze_search_intent(self, prompt: str) -> Dict[str, Any]:
        """Analyze simulation prompt to guide search (Fallback)"""
        prompt_lower = prompt.lower()
        keywords = []
        
        if "opinion" in prompt_lower or "react" in prompt_lower:
            keywords.append("public opinion")
            keywords.append("reaction")
        if "news" in prompt_lower or "article" in prompt_lower:
            keywords.append("news")
        if "video" in prompt_lower:
            keywords.append("video")
        if "scandal" in prompt_lower:
            keywords.append("scandal")
            
        if not keywords:
            keywords = ["latest news", "controversy"]
            
        return {
            "keywords": keywords
        }

    async def _execute_search(
        self,
        client: LightPandaClient,
        crawler: GenericWebCrawler,
        query: str,
        limit: int = 5
    ) -> List[ScrapedPost]:
        """Performs search on DuckDuckGo"""
        posts = []
        search_url = f"https://html.duckduckgo.com/html/?q={quote_plus(query)}"
        
        try:
            logger.info(f"Executing Query: {query}")
            page = await client.get_page()
            await page.goto(search_url)
            await asyncio.sleep(2)
            
            links = []
            link_elements = await page.query_selector_all('.result__a')
            for el in link_elements[:limit]:
                href = await el.get_attribute('href')
                if href and 'duckduckgo' not in href:
                    links.append(href)
            
            for url in links:
                if len(posts) >= limit: break
                try:
                    post = await crawler.scrape_url(url)
                    if post:
                        post.media_type = "text" # Default
                        posts.append(post)
                except Exception: pass
                    
        except Exception as e:
            logger.warning(f"Search failed for {query}: {e}")
        
        return posts

    def _analyze_posts_content(self, posts: List[ScrapedPost]) -> List[ScrapedPost]:
        """Analyze posts for AI generation"""
        for post in posts:
            analysis = {"is_likely_ai": False, "confidence": 0.0, "reason": []}
            content = (post.content or "").lower()
            
            ai_phrases = ["as an ai", "cannot predict", "my knowledge cutoff"]
            matches = [p for p in ai_phrases if p in content]
            if matches:
                analysis["is_likely_ai"] = True
                analysis["confidence"] = 0.9
                analysis["reason"].append(f"AI phrases: {matches}")
            
            post.content_analysis = analysis
        return posts
    
    async def scrape_entity_names(
        self,
        entity_names: List[str],
        simulation_requirement: str = "",
        document_text: str = "",
        max_total_posts: int = 50,
        progress_callback: Optional[callable] = None
    ) -> RealDataSeed:
        """Call with list of names"""
        entities = [{"name": name} for name in entity_names]
        return await self.scrape_entities(
            entities, simulation_requirement, document_text, max_total_posts, progress_callback
        )

