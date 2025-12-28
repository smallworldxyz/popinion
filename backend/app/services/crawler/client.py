"""
LightPanda CDP Client
Chrome DevTools Protocol client for connecting to LightPanda headless browser
"""

import asyncio
from typing import Optional, Any
from contextlib import asynccontextmanager

from ...utils.logger import get_logger

logger = get_logger('pubop.crawler.client')


class LightPandaClient:
    """
    Chrome DevTools Protocol client for LightPanda headless browser.
    
    Provides async connection management and page creation for web scraping.
    Uses Playwright for CDP communication.
    
    Usage:
        async with LightPandaClient() as client:
            page = await client.new_page()
            await page.goto("https://example.com")
            content = await page.content()
    """
    
    def __init__(
        self, 
        endpoint: str = "http://localhost:9222",
        timeout: int = 30000
    ):
        """
        Initialize LightPanda client.
        
        Args:
            endpoint: CDP WebSocket endpoint URL
            timeout: Default timeout for operations in milliseconds
        """
        self.endpoint = endpoint
        self.timeout = timeout
        self._playwright = None
        self._browser = None
        self._connected = False
    
    async def connect(self) -> None:
        """Connect to LightPanda via CDP"""
        if self._connected:
            logger.warning("Already connected to LightPanda")
            return
        
        try:
            # Import playwright here to avoid import errors if not installed
            from playwright.async_api import async_playwright
            
            logger.info(f"Connecting to LightPanda at {self.endpoint}")
            self._playwright = await async_playwright().start()
            self._browser = await self._playwright.chromium.connect_over_cdp(
                self.endpoint,
                timeout=self.timeout
            )
            self._connected = True
            logger.info("Successfully connected to LightPanda")
            
        except Exception as e:
            logger.error(f"Failed to connect to LightPanda: {e}")
            await self.close()
            raise ConnectionError(f"Failed to connect to LightPanda at {self.endpoint}: {e}")
    
    async def close(self) -> None:
        """Close connection to LightPanda"""
        if self._browser:
            try:
                await self._browser.close()
            except Exception as e:
                logger.warning(f"Error closing browser: {e}")
            self._browser = None
        
        if self._playwright:
            try:
                await self._playwright.stop()
            except Exception as e:
                logger.warning(f"Error stopping playwright: {e}")
            self._playwright = None
        
        self._connected = False
        logger.info("Disconnected from LightPanda")
    
    async def new_page(self) -> Any:
        """
        Create a new browser page.
        
        Returns:
            Playwright Page object
        """
        if not self._connected or not self._browser:
            raise ConnectionError("Not connected to LightPanda. Call connect() first.")
        
        context = await self._browser.new_context(
            user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        )
        page = await context.new_page()
        page.set_default_timeout(self.timeout)
        return page
    
    async def new_context(self) -> Any:
        """
        Create a new browser context.
        
        Returns:
            Playwright BrowserContext object
        """
        if not self._connected or not self._browser:
            raise ConnectionError("Not connected to LightPanda. Call connect() first.")
        
        return await self._browser.new_context(
            user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        )
    
    @property
    def is_connected(self) -> bool:
        """Check if connected to LightPanda"""
        return self._connected
    
    async def __aenter__(self) -> "LightPandaClient":
        """Async context manager entry"""
        await self.connect()
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        """Async context manager exit"""
        await self.close()


@asynccontextmanager
async def get_lightpanda_client(
    endpoint: str = "http://localhost:9222",
    timeout: int = 30000
):
    """
    Context manager for LightPanda client.
    
    Usage:
        async with get_lightpanda_client() as client:
            page = await client.new_page()
            ...
    """
    client = LightPandaClient(endpoint=endpoint, timeout=timeout)
    try:
        await client.connect()
        yield client
    finally:
        await client.close()
