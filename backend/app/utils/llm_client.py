"""
LLM client wrapper
Unified API calls using LiteLLM (Supports OpenAI, Gemini, Claude, etc.)
"""

import json
from typing import Optional, Dict, Any, List
import litellm
from litellm import completion

from ..config import Config

class LLMClient:
    """LLM Client using LiteLLM"""
    
    def __init__(
        self,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None
    ):
        self.api_key = api_key or Config.LLM_API_KEY
        self.base_url = base_url or Config.LLM_BASE_URL
        self.model = model or Config.LLM_MODEL_NAME
        
        # BYOK Logic: Check Request Headers
        try:
            from flask import request, has_request_context
            if has_request_context():
                # Provider specific logic could be handled here if needed
                # For now, we expect the frontend to send the key.
                # Ideally, Frontend sends "X-LLM-Provider" to help us choose the model prefix if needed,
                # But typically the Model Name itself implies the provider (gemini/..., anthropic/...)
                
                # Check for generic "X-LLM-Key" or provider specific
                # We will standardize on X-LLM-Key in client.js for the active provider
                header_key = request.headers.get('X-LLM-Key')
                if header_key:
                    self.api_key = header_key
                
                # Allow overriding model from header (if user selects a different model in settings)
                header_model = request.headers.get('X-LLM-Model')
                if header_model:
                    self.model = header_model
                    
        except ImportError:
            pass

        if not self.api_key and not Config.DESKTOP_MODE:
             # In Server Mode, strict check
             raise ValueError("LLM_API_KEY not configured")
             
    
    def chat(
        self,
        messages: List[Dict[str, str]],
        temperature: float = 0.7,
        max_tokens: int = 4096,
        response_format: Optional[Dict] = None
    ) -> str:
        """
        Send chat request
        """
        # LiteLLM handles retries, but we can wrap it if needed.
        # It also handles different providers seamlessly.
        
        try:
            # LiteLLM parameters
            kwargs = {
                "model": self.model,
                "messages": messages,
                "temperature": temperature,
                "max_tokens": max_tokens,
                "api_key": self.api_key,
                "base_url": self.base_url if "openai" in self.model or self.model.startswith("gpt") else None # Only use base_url for OpenAI compat
            }
            
            if response_format:
                kwargs["response_format"] = response_format
            
            # Call LiteLLM
            response = completion(**kwargs)
            
            content = response.choices[0].message.content
            return content if content is not None else ""
            
        except Exception as e:
            from ..utils.logger import get_logger
            logger = get_logger('pubop.llm_client')
            logger.error(f"LiteLLM Error: {str(e)}")
            raise

    def chat_json(
        self,
        messages: List[Dict[str, str]],
        temperature: float = 0.3,
        max_tokens: int = 8192
    ) -> Dict[str, Any]:
        """
        Send chat request and return JSON
        """
        from ..utils.logger import get_logger
        logger = get_logger('pubop.llm_client')
        
        try:
            response_text = self.chat(
                messages=messages,
                temperature=temperature,
                max_tokens=max_tokens,
                response_format={"type": "json_object"}
            )
            
            try:
                return json.loads(response_text)
            except json.JSONDecodeError:
                # Repair logic
                repaired = self._repair_truncated_json(response_text)
                return json.loads(repaired)
                
        except Exception as e:
            logger.error(f"Chat JSON failed: {e}")
            raise

    def _repair_truncated_json(self, content: str) -> str:
        """
        Attempt to repair truncated JSON
        """
        import re
        content = content.strip()
        open_braces = content.count('{') - content.count('}')
        open_brackets = content.count('[') - content.count(']')
        if content and content[-1] not in '",}]':
            if re.search(r':\s*"[^"]*$', content):
                content += '"'
        content += ']' * open_brackets
        content += '}' * open_braces
        return content
