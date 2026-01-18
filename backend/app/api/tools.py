"""
Director Tools API
Tools for the 'Director Mode' such as live search.
"""

import traceback
from flask import Blueprint, request, jsonify
from ..services.pubop_realworld_scraper import RealWorldScraper
from ..utils.logger import get_logger

tools_bp = Blueprint('tools', __name__)
logger = get_logger('pubop.api.tools')

@tools_bp.route('/live-search', methods=['POST'])
async def live_search():
    """
    Perform a live search for real-world context using Playwright/LightPanda.
    
    Request (JSON):
        {
            "query": "Bitcoin price crash",  # Required
            "limit": 5                       # Optional, default 5
        }
    
    Returns:
        {
            "success": true,
            "data": {
                "query": "Bitcoin price crash",
                "results": [
                    {
                        "content": "...",
                        "source_url": "...",
                        "platform": "web",
                        "media_type": "text/image"
                        ...
                    }
                ]
            }
        }
    """
    try:
        data = request.get_json() or {}
        query = data.get('query')
        limit = data.get('limit', 5)
        
        if not query:
            return jsonify({
                "success": False,
                "error": "Query is required"
            }), 400
            
        logger.info(f"Director executing live search: {query}")
        
        # Initialize scraper
        scraper = RealWorldScraper(engine="browserless")
        
        # Use existing logic but adapted for direct query
        # We need a client context, so we'll reuse the internal methods
        from ..services.crawler import LightPandaClient, GenericWebCrawler
        
        posts = []
        async with LightPandaClient(engine=scraper.engine, timeout=scraper.timeout) as client:
            crawler = GenericWebCrawler(client)
            posts = await scraper._execute_search(client, crawler, query, limit=limit)
            
            # Analyze content
            if posts:
                posts = scraper._analyze_posts_content(posts)
        
        # Serialize results
        results = []
        for post in posts:
            results.append({
                "poster_agent_id": 99999,
                "username": post.author_name or "scraped_source",
                "content": post.content[:2000],
                "created_at": post.timestamp.isoformat() if post.timestamp else None,
                "source_url": post.url,
                "platform": post.platform,
                "media_type": getattr(post, "media_type", "text"),
                "content_analysis": getattr(post, "content_analysis", {})
            })
            
        return jsonify({
            "success": True,
            "data": {
                "query": query,
                "count": len(results),
                "results": results
            }
        })

    except Exception as e:
        logger.error(f"Live search failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500
