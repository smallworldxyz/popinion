"""
LLM client wrapper
Unified OpenAI-format API calls
"""

import json
from typing import Optional, Dict, Any, List
from openai import OpenAI

from ..config import Config


class LLMClient:
    """LLM Client"""
    
    def __init__(
        self,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None
    ):
        self.api_key = api_key or Config.LLM_API_KEY
        self.base_url = base_url or Config.LLM_BASE_URL
        self.model = model or Config.LLM_MODEL_NAME
        
        if not self.api_key:
            raise ValueError("LLM_API_KEY not configured")
        
        self.client = OpenAI(
            api_key=self.api_key,
            base_url=self.base_url
        )
    
    def chat(
        self,
        messages: List[Dict[str, str]],
        temperature: float = 0.7,
        max_tokens: int = 4096,
        response_format: Optional[Dict] = None
    ) -> str:
        """
        Send chat request with retry logic
        """
        import time
        import re
        import random
        from openai import RateLimitError
        
        max_retries = 10  # Increased for better resilience
        base_delay = 10.0
        
        for attempt in range(max_retries):
            try:
                kwargs = {
                    "model": self.model,
                    "messages": messages,
                    "temperature": temperature,
                    "max_tokens": max_tokens,
                }
                
                if response_format:
                    kwargs["response_format"] = response_format
                
                response = self.client.chat.completions.create(**kwargs)
                content = response.choices[0].message.content
                return content if content is not None else ""
                
            except RateLimitError as e:
                from ..utils.logger import get_logger
                logger = get_logger('pubop.llm_client')
                
                if attempt < max_retries - 1:
                    # Try to extract retry delay from API error message
                    error_str = str(e)
                    retry_match = re.search(r'retry in (\d+\.?\d*)s', error_str)
                    
                    if retry_match:
                        # Use API-provided delay + jitter
                        api_delay = float(retry_match.group(1))
                        wait_time = api_delay + random.uniform(0.5, 2.0)
                        logger.warning(
                            f"LLM Rate Limit (429) hit. API says wait {api_delay:.1f}s, "
                            f"waiting {wait_time:.1f}s (with jitter)... (Attempt {attempt + 1}/{max_retries})"
                        )
                    else:
                        # Fallback to exponential backoff with jitter
                        wait_time = base_delay * (2 ** attempt) + random.uniform(0.5, 2.0)
                        wait_time = min(wait_time, 120.0)  # Cap at 2 minutes
                        logger.warning(
                            f"LLM Rate Limit (429) hit, retrying in {wait_time:.1f}s... "
                            f"(Attempt {attempt + 1}/{max_retries})"
                        )
                    
                    time.sleep(wait_time)
                else:
                    logger.error(f"LLM Rate Limit exceeded after {max_retries} attempts: {e}")
                    raise
            except Exception as e:
                raise
    
    def chat_json(
        self,
        messages: List[Dict[str, str]],
        temperature: float = 0.3,
        max_tokens: int = 8192  # Increased from 4096 to handle complex responses
    ) -> Dict[str, Any]:
        """
        Send chat request and return JSON
        
        Args:
            messages: Message list
            temperature: Temperature parameter
            max_tokens: Maximum token count
            
        Returns:
            Parsed JSON object
        """
        from ..utils.logger import get_logger
        logger = get_logger('pubop.llm_client')
        
        try:
            logger.debug(f"Calling LLM API: model={self.model}, base_url={self.base_url}")
            
            response = self.chat(
                messages=messages,
                temperature=temperature,
                max_tokens=max_tokens,
                response_format={"type": "json_object"}
            )
            
            logger.debug(f"LLM response received, length: {len(response)} chars")
            
            try:
                parsed = json.loads(response)
                return parsed
            except json.JSONDecodeError as e:
                # Try to repair truncated JSON
                logger.warning(f"JSON parse failed, attempting repair: {e}")
                repaired = self._repair_truncated_json(response)
                return json.loads(repaired)
            
        except json.JSONDecodeError as e:
            logger.error(f"Failed to parse LLM response as JSON: {e}")
            logger.error(f"Response text: {response[:500] if 'response' in locals() else 'No response'}")
            raise ValueError(f"LLM returned invalid JSON: {e}")
        except Exception as e:
            logger.error(f"LLM API call failed: {type(e).__name__}: {str(e)}")
            raise
    
    def _repair_truncated_json(self, content: str) -> str:
        """
        Attempt to repair truncated JSON by closing unclosed brackets
        """
        import re
        
        content = content.strip()
        
        # Count unclosed brackets
        open_braces = content.count('{') - content.count('}')
        open_brackets = content.count('[') - content.count(']')
        
        # Check for unclosed string - if last meaningful char isn't a quote, comma, bracket
        if content and content[-1] not in '",}]':
            # Try to find if we're inside a string value
            # Look for the pattern ": "value that's incomplete
            if re.search(r':\s*"[^"]*$', content):
                content += '"'
        
        # Close brackets and braces
        content += ']' * open_brackets
        content += '}' * open_braces
        
        return content
