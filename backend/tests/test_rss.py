
import asyncio
import sys
import os

# Ensure backend package is in path
sys.path.append(os.getcwd())

from backend.app.services.crawler.rss import RSSCrawler
from backend.app.services.crawler.client import LightPandaClient

async def test_rss():
    url = "http://feeds.bbci.co.uk/news/rss.xml"
    print(f"Testing RSS scraping for: {url}")
    
    async with LightPandaClient(engine="browserless") as client:
        crawler = RSSCrawler(client)
        posts = await crawler.scrape_posts(url, limit=5)
        
        print(f"Found {len(posts)} posts")
        for p in posts:
            print(f"- [{p.timestamp}] {p.content[:50]}...")

if __name__ == "__main__":
    try:
        asyncio.run(test_rss())
    except Exception as e:
        print(f"Test failed: {e}")
