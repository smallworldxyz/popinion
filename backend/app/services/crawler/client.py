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
        """Connect to headless browser via CDP"""
        if self._connected:
            logger.warning("Already connected to headless browser")
            return
        
        try:
            # Import playwright here to avoid import errors if not installed
            from playwright.async_api import async_playwright
            import aiohttp
            
            # Get WebSocket URL from CDP endpoint
            ws_url = await self._get_websocket_url()
            
            logger.info(f"Connecting to headless browser at {ws_url}")
            self._playwright = await async_playwright().start()
            self._browser = await self._playwright.chromium.connect_over_cdp(
                ws_url,
                timeout=self.timeout
            )
            self._connected = True
            logger.info("Successfully connected to headless browser")
            
        except Exception as e:
            logger.error(f"Failed to connect to headless browser: {e}")
            await self.close()
            raise ConnectionError(f"Failed to connect to headless browser at {self.endpoint}: {e}")
    
    async def _get_websocket_url(self) -> str:
        """Get WebSocket URL from CDP endpoint"""
        import aiohttp
        
        # Try to get WS URL from /json/version endpoint
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(f"{self.endpoint}/json/version") as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        ws_url = data.get("webSocketDebuggerUrl", "")
                        if ws_url:
                            # Fix localhost reference for Docker
                            if "localhost:9222" not in ws_url and "localhost:3000" in ws_url:
                                ws_url = ws_url.replace("localhost:3000", "localhost:9222")
                            logger.info(f"Discovered WebSocket URL: {ws_url}")
                            return ws_url
        except Exception as e:
            logger.warning(f"Could not get WS URL from /json/version: {e}")
        
        # Fallback: assume it's a direct WS URL or construct one
        if self.endpoint.startswith("ws://") or self.endpoint.startswith("wss://"):
            return self.endpoint
        
        return f"ws://{self.endpoint.replace('http://', '').replace('https://', '')}"
    
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
