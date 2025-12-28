"""
Crawl API Routes
Endpoints for triggering web scraping via RWSP pipeline
"""

import os
import asyncio
import uuid
from datetime import datetime
from typing import Optional, Dict, Any

from flask import request, jsonify, current_app

from . import crawl_bp
from ..models.pubop import CrawlResult
from ..services.crawler import LightPandaClient, TelegramCrawler, TwitterCrawler, FacebookCrawler
from ..services.pubop_bridge import PubopBridge, RealDataSeed
from ..config import Config
from ..utils.logger import get_logger

logger = get_logger('pubop.api.crawl')

# Store active crawl jobs (in production, use Redis or database)
_crawl_jobs: Dict[str, Dict[str, Any]] = {}


def _get_data_dir() -> str:
    """Get data directory for storing crawl results"""
    data_dir = os.path.join(Config.DATA_DIR, "crawls")
    os.makedirs(data_dir, exist_ok=True)
    return data_dir


def _run_async(coro):
    """Run async function in sync context"""
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        return loop.run_until_complete(coro)
    finally:
        loop.close()


@crawl_bp.route('/telegram', methods=['POST'])
def crawl_telegram():
    """
    Crawl a Telegram channel
    
    Request body:
    {
        "channel": "durov",  // Channel username (without @)
        "max_posts": 50,     // Optional, default 50
        "save_result": true  // Optional, save to file
    }
    
    Returns:
        Crawl result with posts, users, and optional seed
    """
    data = request.get_json() or {}
    channel = data.get('channel')
    max_posts = data.get('max_posts', 50)
    save_result = data.get('save_result', True)
    create_seed = data.get('create_seed', False)
    
    if not channel:
        return jsonify({"error": "channel is required"}), 400
    
    try:
        async def do_crawl():
            async with LightPandaClient() as client:
                crawler = TelegramCrawler(client)
                return await crawler.scrape_channel(channel, max_posts=max_posts)
        
        result = _run_async(do_crawl())
        
        # Save if requested
        filepath = None
        if save_result:
            filename = f"telegram_{channel}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
            filepath = os.path.join(_get_data_dir(), filename)
            result.save(filepath)
            logger.info(f"Saved crawl result to {filepath}")
        
        response = {
            "success": result.success,
            "platform": result.platform,
            "channel": channel,
            "posts_count": len(result.posts),
            "users_count": len(result.users),
            "crawled_at": result.crawled_at.isoformat(),
        }
        
        if filepath:
            response["saved_to"] = filepath
        
        # Create seed if requested
        if create_seed:
            bridge = PubopBridge()
            seed = bridge.create_seed_from_crawl(result)
            response["seed"] = seed.to_dict()
        
        # Include sample posts
        response["sample_posts"] = [p.to_dict() for p in result.posts[:5]]
        
        return jsonify(response)
        
    except Exception as e:
        logger.error(f"Telegram crawl failed: {e}")
        return jsonify({"error": str(e)}), 500


@crawl_bp.route('/twitter', methods=['POST'])
def crawl_twitter():
    """
    Crawl Twitter/X
    
    Request body:
    {
        "query": "AI",           // Search query OR
        "username": "elonmusk",  // Username to scrape OR
        "trending": true,        // Get trending topics
        "max_posts": 50,
        "save_result": true
    }
    """
    data = request.get_json() or {}
    query = data.get('query')
    username = data.get('username')
    trending = data.get('trending', False)
    max_posts = data.get('max_posts', 50)
    save_result = data.get('save_result', True)
    create_seed = data.get('create_seed', False)
    
    if not query and not username and not trending:
        return jsonify({"error": "query, username, or trending=true is required"}), 400
    
    try:
        async def do_crawl():
            async with LightPandaClient() as client:
                crawler = TwitterCrawler(client)
                
                if trending:
                    trends = await crawler.scrape_trending()
                    return CrawlResult(
                        platform="twitter",
                        query="trending",
                        trends=trends
                    )
                elif username:
                    user = await crawler.scrape_user(username)
                    posts = await crawler.scrape_posts(username, max_posts=max_posts)
                    return CrawlResult(
                        platform="twitter",
                        query=f"@{username}",
                        posts=posts,
                        users=[user] if user else []
                    )
                else:
                    posts = await crawler.scrape_posts(query, max_posts=max_posts)
                    return CrawlResult(
                        platform="twitter",
                        query=query,
                        posts=posts
                    )
        
        result = _run_async(do_crawl())
        
        # Save if requested
        filepath = None
        if save_result:
            safe_name = (query or username or "trending").replace(" ", "_")[:30]
            filename = f"twitter_{safe_name}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
            filepath = os.path.join(_get_data_dir(), filename)
            result.save(filepath)
        
        response = {
            "success": result.success,
            "platform": result.platform,
            "query": result.query,
            "posts_count": len(result.posts),
            "trends_count": len(result.trends),
            "users_count": len(result.users),
            "crawled_at": result.crawled_at.isoformat(),
        }
        
        if filepath:
            response["saved_to"] = filepath
        
        if create_seed and result.posts:
            bridge = PubopBridge()
            seed = bridge.create_seed_from_crawl(result)
            response["seed"] = seed.to_dict()
        
        if result.trends:
            response["trends"] = [t.to_dict() for t in result.trends[:10]]
        
        response["sample_posts"] = [p.to_dict() for p in result.posts[:5]]
        
        return jsonify(response)
        
    except Exception as e:
        logger.error(f"Twitter crawl failed: {e}")
        return jsonify({"error": str(e)}), 500


@crawl_bp.route('/facebook', methods=['POST'])
def crawl_facebook():
    """
    Crawl a Facebook page
    
    Request body:
    {
        "page": "Meta",      // Page name/ID
        "max_posts": 50,
        "save_result": true
    }
    """
    data = request.get_json() or {}
    page = data.get('page')
    max_posts = data.get('max_posts', 50)
    save_result = data.get('save_result', True)
    create_seed = data.get('create_seed', False)
    
    if not page:
        return jsonify({"error": "page is required"}), 400
    
    try:
        async def do_crawl():
            async with LightPandaClient() as client:
                crawler = FacebookCrawler(client)
                return await crawler.scrape_channel(page, max_posts=max_posts)
        
        result = _run_async(do_crawl())
        
        filepath = None
        if save_result:
            filename = f"facebook_{page}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
            filepath = os.path.join(_get_data_dir(), filename)
            result.save(filepath)
        
        response = {
            "success": result.success,
            "platform": result.platform,
            "page": page,
            "posts_count": len(result.posts),
            "users_count": len(result.users),
            "crawled_at": result.crawled_at.isoformat(),
        }
        
        if filepath:
            response["saved_to"] = filepath
        
        if create_seed and result.posts:
            bridge = PubopBridge()
            seed = bridge.create_seed_from_crawl(result)
            response["seed"] = seed.to_dict()
        
        response["sample_posts"] = [p.to_dict() for p in result.posts[:5]]
        
        return jsonify(response)
        
    except Exception as e:
        logger.error(f"Facebook crawl failed: {e}")
        return jsonify({"error": str(e)}), 500


@crawl_bp.route('/results', methods=['GET'])
def list_crawl_results():
    """
    List saved crawl results
    
    Query params:
        platform: Filter by platform (telegram/twitter/facebook)
        limit: Max results (default 20)
    """
    platform = request.args.get('platform')
    limit = int(request.args.get('limit', 20))
    
    data_dir = _get_data_dir()
    
    results = []
    for filename in sorted(os.listdir(data_dir), reverse=True)[:limit]:
        if not filename.endswith('.json'):
            continue
        
        if platform and not filename.startswith(platform):
            continue
        
        filepath = os.path.join(data_dir, filename)
        stat = os.stat(filepath)
        
        results.append({
            "filename": filename,
            "filepath": filepath,
            "size_bytes": stat.st_size,
            "created_at": datetime.fromtimestamp(stat.st_ctime).isoformat(),
        })
    
    return jsonify({
        "count": len(results),
        "results": results
    })


@crawl_bp.route('/results/<filename>', methods=['GET'])
def get_crawl_result(filename: str):
    """Load a saved crawl result"""
    filepath = os.path.join(_get_data_dir(), filename)
    
    if not os.path.exists(filepath):
        return jsonify({"error": "File not found"}), 404
    
    try:
        result = CrawlResult.load(filepath)
        return jsonify(result.to_dict())
    except Exception as e:
        return jsonify({"error": str(e)}), 500


@crawl_bp.route('/bridge', methods=['POST'])
def bridge_to_simulation():
    """
    Bridge a saved crawl result to simulation format
    
    Request body:
    {
        "filepath": "/path/to/crawl.json",  // OR
        "filename": "telegram_durov_xxx.json",
        "anonymize": true,
        "max_profiles": 100
    }
    """
    data = request.get_json() or {}
    filepath = data.get('filepath')
    filename = data.get('filename')
    anonymize = data.get('anonymize', True)
    max_profiles = data.get('max_profiles', 100)
    
    if not filepath and not filename:
        return jsonify({"error": "filepath or filename is required"}), 400
    
    if filename and not filepath:
        filepath = os.path.join(_get_data_dir(), filename)
    
    if not os.path.exists(filepath):
        return jsonify({"error": "File not found"}), 404
    
    try:
        # Load crawl result
        result = CrawlResult.load(filepath)
        
        # Bridge to simulation format
        bridge = PubopBridge(anonymize=anonymize)
        seed = bridge.create_seed_from_crawl(result, max_profiles=max_profiles)
        
        return jsonify({
            "success": True,
            "platform": seed.platform,
            "profiles_count": len(seed.profiles),
            "initial_posts_count": len(seed.initial_posts),
            "trending_topics": seed.trending_topics,
            "seed": seed.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Bridge failed: {e}")
        return jsonify({"error": str(e)}), 500
