"""
LightPanda CDP Client
Chrome DevTools Protocol client for connecting to headless browser (Browserless Chrome)
"""

import os
import asyncio
import random
from typing import Optional, Any, List, Dict
from contextlib import asynccontextmanager

from ...utils.logger import get_logger

logger = get_logger('pubop.crawler.client')


class ProxyConfig:
    """
    Proxy configuration for anti-bot protection.
    
    Supports single proxy or proxy rotation pool.
    
    Usage:
        # Single proxy
        proxy = ProxyConfig(server="http://proxy.example.com:8080")
        
        # Proxy with auth
        proxy = ProxyConfig(
            server="http://proxy.example.com:8080",
            username="user",
            password="pass"
        )
        
        # Proxy rotation pool
        proxy = ProxyConfig(pool=[
            "http://proxy1.example.com:8080",
            "http://proxy2.example.com:8080",
        ])
        
        # From environment
        proxy = ProxyConfig.from_env()
    """
    
    def __init__(
        self,
        server: Optional[str] = None,
        username: Optional[str] = None,
        password: Optional[str] = None,
        pool: Optional[List[str]] = None,
    ):
        self.server = server
        self.username = username
        self.password = password
        self.pool = pool or []
        self._current_index = 0
    
    @classmethod
    def from_env(cls) -> Optional['ProxyConfig']:
        """
        Create ProxyConfig from environment variables.
        
        Environment variables:
            CRAWLER_PROXY_SERVER: Single proxy server URL
            CRAWLER_PROXY_USERNAME: Proxy username (optional)
            CRAWLER_PROXY_PASSWORD: Proxy password (optional)
            CRAWLER_PROXY_POOL: Comma-separated list of proxy servers
        """
        server = os.getenv("CRAWLER_PROXY_SERVER")
        pool_str = os.getenv("CRAWLER_PROXY_POOL", "")
        
        if not server and not pool_str:
            return None
        
        pool = [p.strip() for p in pool_str.split(",") if p.strip()] if pool_str else []
        
        return cls(
            server=server,
            username=os.getenv("CRAWLER_PROXY_USERNAME"),
            password=os.getenv("CRAWLER_PROXY_PASSWORD"),
            pool=pool,
        )
    
    def get_proxy(self, rotate: bool = True) -> Optional[Dict[str, str]]:
        """
        Get proxy configuration for Playwright.
        
        Args:
            rotate: If True and pool exists, rotate to next proxy
            
        Returns:
            Dict with server, username, password for Playwright context
        """
        server = self.server
        
        # Use pool if available
        if self.pool:
            if rotate:
                self._current_index = (self._current_index + 1) % len(self.pool)
            server = self.pool[self._current_index]
        
        if not server:
            return None
        
        config = {"server": server}
        if self.username:
            config["username"] = self.username
        if self.password:
            config["password"] = self.password
        
        return config
    
    def get_random_proxy(self) -> Optional[Dict[str, str]]:
        """Get a random proxy from pool"""
        if self.pool:
            server = random.choice(self.pool)
            config = {"server": server}
            if self.username:
                config["username"] = self.username
            if self.password:
                config["password"] = self.password
            return config
        return self.get_proxy(rotate=False)


class LightPandaClient:
    """
    Chrome DevTools Protocol client for headless browser.
    
    Provides async connection management and page creation for web scraping.
    Uses Playwright for CDP communication.
    
    Usage:
        # Basic usage (LightPanda by default)
        async with LightPandaClient() as client:
            page = await client.new_page()
            await page.goto("https://example.com")
            content = await page.content()
        
        # Use Browserless fallback
        async with LightPandaClient(engine="browserless") as client:
            ...
        
        # With proxy
        proxy = ProxyConfig(server="http://proxy:8080")
        async with LightPandaClient(proxy=proxy) as client:
            page = await client.new_page()
            ...
    """
    
    # Engine endpoints
    ENGINES = {
        "lightpanda": "http://localhost:9222",
        "browserless": "http://localhost:9223",
    }
    
    # User agent pool for rotation
    USER_AGENTS = [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    ]
    
    def __init__(
        self, 
        endpoint: str = None,
        timeout: int = 30000,
        proxy: Optional[ProxyConfig] = None,
        rotate_user_agent: bool = True,
        engine: str = None,
        fallback_endpoint: str = None,
    ):
        """
        Initialize client.
        
        Args:
            endpoint: CDP WebSocket endpoint URL (overrides engine)
            timeout: Default timeout for operations in milliseconds
            proxy: Optional ProxyConfig for anti-bot protection
            rotate_user_agent: Rotate user agent for each context
            engine: Browser engine to use: "lightpanda" (default) or "browserless"
            fallback_endpoint: Fallback CDP endpoint if primary fails
        """
        # Determine engine from environment or default
        engine = engine or os.getenv("CRAWLER_ENGINE", "lightpanda")
        
        # Set endpoint based on engine or explicit override
        if endpoint:
            self.endpoint = endpoint
        else:
            self.endpoint = os.getenv("CRAWLER_CDP_ENDPOINT") or self.ENGINES.get(engine, self.ENGINES["lightpanda"])
        
        # Fallback endpoint (default to browserless if using lightpanda)
        if fallback_endpoint:
            self.fallback_endpoint = fallback_endpoint
        elif engine == "lightpanda":
            self.fallback_endpoint = self.ENGINES["browserless"]
        else:
            self.fallback_endpoint = None
        
        self.engine = engine
        self.timeout = timeout
        self.proxy = proxy
        self.rotate_user_agent = rotate_user_agent
        self._playwright = None
        self._browser = None
        self._connected = False
    
    def _get_user_agent(self) -> str:
        """Get user agent (random if rotating)"""
        if self.rotate_user_agent:
            return random.choice(self.USER_AGENTS)
        return self.USER_AGENTS[0]
    
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
        """Close connection"""
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
        logger.info("Disconnected from headless browser")
    
    async def new_page(self, use_proxy: bool = True) -> Any:
        """
        Create a new browser page.
        
        Args:
            use_proxy: Whether to use proxy if configured
        
        Returns:
            Playwright Page object
        """
        if not self._connected or not self._browser:
            raise ConnectionError("Not connected. Call connect() first.")
        
        context_kwargs = {}
        
        # LightPanda doesn't support setUserAgentOverride
        if self.engine != "lightpanda":
            context_kwargs["user_agent"] = self._get_user_agent()
        
        # Add proxy if configured (may not work with LightPanda)
        if use_proxy and self.proxy and self.engine != "lightpanda":
            proxy_config = self.proxy.get_proxy(rotate=True)
            if proxy_config:
                context_kwargs["proxy"] = proxy_config
                logger.debug(f"Using proxy: {proxy_config['server']}")
        
        context = await self._browser.new_context(**context_kwargs)
        page = await context.new_page()
        page.set_default_timeout(self.timeout)
        return page
    
    async def new_context(self, use_proxy: bool = True) -> Any:
        """
        Create a new browser context.
        
        Args:
            use_proxy: Whether to use proxy if configured
        
        Returns:
            Playwright BrowserContext object
        """
        if not self._connected or not self._browser:
            raise ConnectionError("Not connected. Call connect() first.")
        
        context_kwargs = {}
        
        # LightPanda doesn't support setUserAgentOverride
        if self.engine != "lightpanda":
            context_kwargs["user_agent"] = self._get_user_agent()
        
        if use_proxy and self.proxy and self.engine != "lightpanda":
            proxy_config = self.proxy.get_proxy(rotate=True)
            if proxy_config:
                context_kwargs["proxy"] = proxy_config
        
        return await self._browser.new_context(**context_kwargs)
    
    @property
    def is_connected(self) -> bool:
        """Check if connected"""
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
    endpoint: str = None,
    timeout: int = 30000,
    proxy: Optional[ProxyConfig] = None,
):
    """
    Context manager for headless browser client.
    
    Usage:
        async with get_lightpanda_client() as client:
            page = await client.new_page()
            ...
        
        # With proxy from environment
        async with get_lightpanda_client(proxy=ProxyConfig.from_env()) as client:
            ...
    """
    client = LightPandaClient(endpoint=endpoint, timeout=timeout, proxy=proxy)
    try:
        await client.connect()
        yield client
    finally:
        await client.close()

