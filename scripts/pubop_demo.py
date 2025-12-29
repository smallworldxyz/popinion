#!/usr/bin/env python3
"""
RWSP Demo Script
Real-World Simulation Prediction end-to-end demonstration

Usage:
    # Crawl Telegram channel
    python scripts/rwsp_demo.py telegram --channel durov --max-posts 20
    
    # Crawl Twitter search
    python scripts/rwsp_demo.py twitter --query "AI news" --max-posts 20
    
    # Bridge crawl result to simulation format
    python scripts/rwsp_demo.py bridge --input crawls/telegram_durov_xxx.json
    
    # Full pipeline: crawl → bridge → show summary
    python scripts/rwsp_demo.py full --platform telegram --channel durov
"""

import os
import sys
import asyncio
import argparse
from datetime import datetime

# Add backend to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'backend'))

from app.models.pubop import CrawlResult
from app.services.crawler import LightPandaClient, TelegramCrawler, TwitterCrawler, FacebookCrawler, InstagramCrawler
from app.services.pubop_bridge import PubopBridge, RealDataSeed


def get_data_dir() -> str:
    """Get data directory for storing crawl results"""
    data_dir = os.path.join(os.path.dirname(__file__), '..', 'data', 'crawls')
    os.makedirs(data_dir, exist_ok=True)
    return data_dir


async def crawl_telegram(channel: str, max_posts: int = 50, save: bool = True, engine: str = None) -> CrawlResult:
    """Crawl a Telegram channel"""
    print(f"\n📱 Crawling Telegram channel: @{channel}")
    print(f"   Max posts: {max_posts}")
    
    engine = engine or "lightpanda"
    async with LightPandaClient(engine=engine) as client:
        print(f"   Engine: {client.engine}")
        crawler = TelegramCrawler(client)
        posts = await crawler.scrape_channel(channel, limit=max_posts)
        result = CrawlResult(
            platform="telegram",
            query=channel,
            posts=posts
        )
    
    # Retry with Browserless if no results and using LightPanda
    if len(result.posts) == 0 and engine == "lightpanda":
        print("   ⚠️  No results with LightPanda, retrying with Browserless...")
        return await crawl_telegram(channel, max_posts, save, engine="browserless")
    
    print(f"   ✅ Found {len(result.posts)} posts")
    
    if save and len(result.posts) > 0:
        filename = f"telegram_{channel}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        filepath = os.path.join(get_data_dir(), filename)
        result.save(filepath)
        print(f"   💾 Saved to: {filepath}")
    
    return result



async def crawl_twitter(query: str = None, username: str = None, max_posts: int = 50, save: bool = True, engine: str = None) -> CrawlResult:
    """Crawl Twitter"""
    target = query or f"@{username}"
    print(f"\n🐦 Crawling Twitter: {target}")
    print(f"   Max posts: {max_posts}")
    
    engine = engine or "lightpanda"
    async with LightPandaClient(engine=engine) as client:
        print(f"   Engine: {client.engine}")
        crawler = TwitterCrawler(client)
        
        if username:
            posts = await crawler.scrape_posts(username, limit=max_posts)
            user = await crawler.scrape_user(username)
            result = CrawlResult(
                platform="twitter",
                query=f"@{username}",
                posts=posts,
                users=[user] if user else []
            )
        else:
            posts = await crawler.scrape_posts(query, limit=max_posts)
            result = CrawlResult(
                platform="twitter",
                query=query,
                posts=posts
            )
    
    # Retry with Browserless if no results and using LightPanda
    if len(result.posts) == 0 and engine == "lightpanda":
        print("   ⚠️  No results with LightPanda, retrying with Browserless...")
        return await crawl_twitter(query, username, max_posts, save, engine="browserless")
    
    print(f"   ✅ Found {len(result.posts)} posts")
    
    if save and len(result.posts) > 0:
        safe_name = (query or username).replace(" ", "_")[:30]
        filename = f"twitter_{safe_name}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        filepath = os.path.join(get_data_dir(), filename)
        result.save(filepath)
        print(f"   💾 Saved to: {filepath}")
    
    return result


async def crawl_facebook(page: str, max_posts: int = 50, save: bool = True, engine: str = None) -> CrawlResult:
    """Crawl a Facebook page"""
    print(f"\n📘 Crawling Facebook page: {page}")
    print(f"   Max posts: {max_posts}")
    
    engine = engine or "lightpanda"
    async with LightPandaClient(engine=engine) as client:
        print(f"   Engine: {client.engine}")
        crawler = FacebookCrawler(client)
        posts = await crawler.scrape_channel(page, limit=max_posts)
        result = CrawlResult(
            platform="facebook",
            query=page,
            posts=posts
        )
    
    # Retry with Browserless if no results and using LightPanda
    if len(result.posts) == 0 and engine == "lightpanda":
        print("   ⚠️  No results with LightPanda, retrying with Browserless...")
        return await crawl_facebook(page, max_posts, save, engine="browserless")
    
    print(f"   ✅ Found {len(result.posts)} posts")
    
    if save and len(result.posts) > 0:
        filename = f"facebook_{page}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        filepath = os.path.join(get_data_dir(), filename)
        result.save(filepath)
        print(f"   💾 Saved to: {filepath}")
    
    return result


async def crawl_instagram(username: str = None, hashtag: str = None, max_posts: int = 20, save: bool = True, engine: str = None) -> CrawlResult:
    """Crawl Instagram profile or hashtag"""
    target = f"@{username}" if username else f"#{hashtag}"
    print(f"\n📸 Crawling Instagram: {target}")
    print(f"   Max posts: {max_posts}")
    
    # Instagram needs Browserless due to complex JS
    engine = engine or "browserless"
    async with LightPandaClient(engine=engine) as client:
        print(f"   Engine: {client.engine}")
        crawler = InstagramCrawler(client)
        
        if username:
            posts = await crawler.scrape_channel(username, limit=max_posts)
        else:
            posts = await crawler.scrape_posts(hashtag, limit=max_posts)
        
        result = CrawlResult(
            platform="instagram",
            query=target,
            posts=posts
        )
    
    print(f"   ✅ Found {len(result.posts)} posts")
    
    if save and len(result.posts) > 0:
        safe_name = (username or hashtag).replace(" ", "_")[:30]
        filename = f"instagram_{safe_name}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        filepath = os.path.join(get_data_dir(), filename)
        result.save(filepath)
        print(f"   💾 Saved to: {filepath}")
    
    return result


def bridge_to_seed(result: CrawlResult, anonymize: bool = True, max_profiles: int = 100) -> RealDataSeed:
    """Bridge crawl result to simulation seed"""
    print(f"\n🔗 Bridging to simulation format...")
    print(f"   Anonymize: {anonymize}")
    print(f"   Max profiles: {max_profiles}")
    
    bridge = PubopBridge(anonymize=anonymize)
    seed = bridge.create_seed_from_crawl(result, max_profiles=max_profiles)
    
    print(f"   ✅ Created seed:")
    print(f"      - Profiles: {len(seed.profiles)}")
    print(f"      - Initial posts: {len(seed.initial_posts)}")
    print(f"      - Trending topics: {len(seed.trending_topics)}")
    
    return seed


def show_summary(result: CrawlResult, seed: RealDataSeed = None):
    """Show summary of crawl and seed"""
    print("\n" + "=" * 60)
    print("📊 CRAWL SUMMARY")
    print("=" * 60)
    print(f"Platform: {result.platform}")
    print(f"Query: {result.query}")
    print(f"Crawled at: {result.crawled_at}")
    print(f"Posts: {len(result.posts)}")
    print(f"Users: {len(result.users)}")
    print(f"Trends: {len(result.trends)}")
    
    if result.posts:
        print("\n📝 Sample posts:")
        for i, post in enumerate(result.posts[:3], 1):
            content = post.content[:80].replace('\n', ' ')
            print(f"   {i}. [{post.author_name}] {content}...")
    
    if seed:
        print("\n" + "=" * 60)
        print("🎯 SIMULATION SEED")
        print("=" * 60)
        print(f"Profiles: {len(seed.profiles)}")
        print(f"Initial posts: {len(seed.initial_posts)}")
        
        if seed.profiles:
            print("\n👥 Sample profiles:")
            for i, profile in enumerate(seed.profiles[:3], 1):
                print(f"   {i}. {profile.user_name}: {profile.persona[:60]}...")
    
    print("\n" + "=" * 60)


async def full_pipeline(platform: str, channel: str = None, query: str = None, max_posts: int = 50):
    """Run full RWSP pipeline: crawl → bridge → summary"""
    print("\n" + "=" * 60)
    print("🚀 RWSP FULL PIPELINE")
    print("=" * 60)
    
    # Step 1: Crawl
    if platform == "telegram":
        result = await crawl_telegram(channel, max_posts=max_posts)
    elif platform == "twitter":
        result = await crawl_twitter(query=query, max_posts=max_posts)
    elif platform == "facebook":
        result = await crawl_facebook(channel, max_posts=max_posts)
    else:
        print(f"❌ Unknown platform: {platform}")
        return
    
    # Step 2: Bridge
    seed = bridge_to_seed(result)
    
    # Step 3: Summary
    show_summary(result, seed)
    
    return result, seed


def main():
    parser = argparse.ArgumentParser(description="RWSP Demo - Real-World Simulation Prediction")
    subparsers = parser.add_subparsers(dest="command", help="Command to run")
    
    # Telegram command
    tg_parser = subparsers.add_parser("telegram", help="Crawl Telegram channel")
    tg_parser.add_argument("--channel", required=True, help="Channel username (without @)")
    tg_parser.add_argument("--max-posts", type=int, default=50, help="Max posts to crawl")
    tg_parser.add_argument("--no-save", action="store_true", help="Don't save to file")
    
    # Twitter command
    tw_parser = subparsers.add_parser("twitter", help="Crawl Twitter")
    tw_parser.add_argument("--query", help="Search query")
    tw_parser.add_argument("--username", help="Username to scrape")
    tw_parser.add_argument("--max-posts", type=int, default=50, help="Max posts to crawl")
    tw_parser.add_argument("--no-save", action="store_true", help="Don't save to file")
    
    # Facebook command
    fb_parser = subparsers.add_parser("facebook", help="Crawl Facebook page")
    fb_parser.add_argument("--page", required=True, help="Page name or ID")
    fb_parser.add_argument("--max-posts", type=int, default=50, help="Max posts to crawl")
    fb_parser.add_argument("--no-save", action="store_true", help="Don't save to file")
    
    # Instagram command
    ig_parser = subparsers.add_parser("instagram", help="Crawl Instagram profile")
    ig_parser.add_argument("--username", help="Profile username (without @)")
    ig_parser.add_argument("--hashtag", help="Hashtag to search (without #)")
    ig_parser.add_argument("--max-posts", type=int, default=20, help="Max posts to crawl")
    ig_parser.add_argument("--no-save", action="store_true", help="Don't save to file")
    
    # Bridge command
    bridge_parser = subparsers.add_parser("bridge", help="Bridge crawl result to simulation")
    bridge_parser.add_argument("--input", required=True, help="Input crawl result JSON file")
    bridge_parser.add_argument("--no-anonymize", action="store_true", help="Don't anonymize users")
    bridge_parser.add_argument("--max-profiles", type=int, default=100, help="Max profiles to generate")
    
    # Full pipeline command
    full_parser = subparsers.add_parser("full", help="Run full RWSP pipeline")
    full_parser.add_argument("--platform", required=True, choices=["telegram", "twitter", "facebook"])
    full_parser.add_argument("--channel", help="Channel/page name (for telegram/facebook)")
    full_parser.add_argument("--query", help="Search query (for twitter)")
    full_parser.add_argument("--max-posts", type=int, default=50, help="Max posts to crawl")
    
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        return
    
    try:
        if args.command == "telegram":
            asyncio.run(crawl_telegram(args.channel, args.max_posts, save=not args.no_save))
        
        elif args.command == "twitter":
            if not args.query and not args.username:
                print("❌ Either --query or --username is required")
                return
            asyncio.run(crawl_twitter(args.query, args.username, args.max_posts, save=not args.no_save))
        
        elif args.command == "facebook":
            asyncio.run(crawl_facebook(args.page, args.max_posts, save=not args.no_save))
        
        elif args.command == "instagram":
            if not args.username and not args.hashtag:
                print("❌ Either --username or --hashtag is required")
                return
            asyncio.run(crawl_instagram(args.username, args.hashtag, args.max_posts, save=not args.no_save))
        
        elif args.command == "bridge":
            result = CrawlResult.load(args.input)
            seed = bridge_to_seed(result, anonymize=not args.no_anonymize, max_profiles=args.max_profiles)
            show_summary(result, seed)
        
        elif args.command == "full":
            if args.platform in ["telegram", "facebook"] and not args.channel:
                print("❌ --channel is required for telegram/facebook")
                return
            if args.platform == "twitter" and not args.query:
                print("❌ --query is required for twitter")
                return
            asyncio.run(full_pipeline(args.platform, args.channel, args.query, args.max_posts))
        
        print("\n✅ Done!")
        
    except KeyboardInterrupt:
        print("\n⚠️ Interrupted")
    except Exception as e:
        print(f"\n❌ Error: {e}")
        raise


if __name__ == "__main__":
    main()
