"""
Panel Chat Service
Aggregates responses from multiple agents for Panel Chat feature

Provides:
1. Response aggregation by stance (support/oppose/neutral)
2. Response grouping by faction (agent type/role)
3. Stance distribution statistics
"""

import json
import os
import re
from typing import Dict, Any, List, Optional, Tuple
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum

from openai import OpenAI

from ..config import Config
from ..utils.logger import get_logger

logger = get_logger('pubop.panel_chat')


class Stance(Enum):
    """Agent stance classification"""
    SUPPORT = "support"
    OPPOSE = "oppose"
    NEUTRAL = "neutral"
    UNKNOWN = "unknown"


@dataclass
class AgentResponse:
    """Individual agent response"""
    agent_id: int
    agent_name: str
    platform: str
    response: str
    stance: Stance = Stance.UNKNOWN
    faction: str = "unknown"
    key_points: List[str] = field(default_factory=list)


@dataclass
class PanelChatResult:
    """Aggregated panel chat result"""
    question: str
    timestamp: str
    total_agents: int
    responses: List[AgentResponse] = field(default_factory=list)
    
    # Grouped views
    by_stance: Dict[str, List[AgentResponse]] = field(default_factory=dict)
    by_faction: Dict[str, List[AgentResponse]] = field(default_factory=dict)
    
    # Statistics
    stance_distribution: Dict[str, float] = field(default_factory=dict)
    stance_counts: Dict[str, int] = field(default_factory=dict)
    faction_counts: Dict[str, int] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization"""
        return {
            "question": self.question,
            "timestamp": self.timestamp,
            "total_agents": self.total_agents,
            "responses": [
                {
                    "agent_id": r.agent_id,
                    "agent_name": r.agent_name,
                    "platform": r.platform,
                    "response": r.response,
                    "stance": r.stance.value,
                    "faction": r.faction,
                    "key_points": r.key_points,
                }
                for r in self.responses
            ],
            "by_stance": {
                stance: [
                    {
                        "agent_id": r.agent_id,
                        "agent_name": r.agent_name,
                        "response": r.response[:200] + "..." if len(r.response) > 200 else r.response,
                        "faction": r.faction,
                    }
                    for r in responses
                ]
                for stance, responses in self.by_stance.items()
            },
            "by_faction": {
                faction: [
                    {
                        "agent_id": r.agent_id,
                        "agent_name": r.agent_name,
                        "response": r.response[:200] + "..." if len(r.response) > 200 else r.response,
                        "stance": r.stance.value,
                    }
                    for r in responses
                ]
                for faction, responses in self.by_faction.items()
            },
            "stance_distribution": self.stance_distribution,
            "stance_counts": self.stance_counts,
            "faction_counts": self.faction_counts,
        }


class PanelChatService:
    """
    Panel Chat Service
    
    Aggregates responses from interview_all_agents and provides:
    - Stance classification (support/oppose/neutral)
    - Faction grouping
    - Statistical analysis
    """
    
    def __init__(self):
        self.api_key = Config.LLM_API_KEY
        self.base_url = Config.LLM_BASE_URL
        self.model_name = Config.LLM_MODEL_NAME
        
        self._llm_client = None
    
    @property
    def llm(self) -> OpenAI:
        """Lazy-initialize LLM client"""
        if self._llm_client is None:
            self._llm_client = OpenAI(
                api_key=self.api_key,
                base_url=self.base_url
            )
        return self._llm_client
    
    def aggregate_responses(
        self,
        question: str,
        raw_results: Dict[str, Any],
        profiles: List[Dict[str, Any]],
        classify_stance: bool = True
    ) -> PanelChatResult:
        """
        Aggregate raw interview responses into structured panel chat result
        
        Args:
            question: The original question asked
            raw_results: Raw results from interview_all_agents
            profiles: List of agent profiles (for faction info)
            classify_stance: Whether to use LLM to classify stance
            
        Returns:
            PanelChatResult with aggregated and grouped responses
        """
        timestamp = datetime.now().isoformat()
        
        # Build profile lookup by agent_id
        profile_lookup = {}
        for p in profiles:
            agent_id = p.get("user_id")
            if agent_id is not None:
                profile_lookup[agent_id] = p
        
        # Parse raw results into AgentResponse objects
        responses = []
        results_data = raw_results.get("results", {})
        
        for key, result in results_data.items():
            if not isinstance(result, dict):
                continue
                
            agent_id = result.get("agent_id")
            response_text = result.get("response", "")
            platform = result.get("platform", "unknown")
            
            # Get faction from profile
            profile = profile_lookup.get(agent_id, {})
            faction = profile.get("source_entity_type", "unknown")
            if not faction or faction == "unknown":
                # Try profession as fallback
                faction = profile.get("profession", "unknown")
            
            agent_name = profile.get("name", f"Agent_{agent_id}")
            
            agent_response = AgentResponse(
                agent_id=agent_id,
                agent_name=agent_name,
                platform=platform,
                response=response_text,
                faction=faction,
            )
            responses.append(agent_response)
        
        # Classify stances if requested
        if classify_stance and responses:
            responses = self._classify_stances(question, responses)
        
        # Group by stance
        by_stance = {stance.value: [] for stance in Stance}
        for r in responses:
            by_stance[r.stance.value].append(r)
        
        # Group by faction
        by_faction: Dict[str, List[AgentResponse]] = {}
        for r in responses:
            faction_key = r.faction or "unknown"
            if faction_key not in by_faction:
                by_faction[faction_key] = []
            by_faction[faction_key].append(r)
        
        # Calculate statistics
        total = len(responses)
        stance_counts = {stance: len(resps) for stance, resps in by_stance.items()}
        stance_distribution = {
            stance: round(count / total * 100, 1) if total > 0 else 0
            for stance, count in stance_counts.items()
        }
        faction_counts = {faction: len(resps) for faction, resps in by_faction.items()}
        
        return PanelChatResult(
            question=question,
            timestamp=timestamp,
            total_agents=total,
            responses=responses,
            by_stance=by_stance,
            by_faction=by_faction,
            stance_distribution=stance_distribution,
            stance_counts=stance_counts,
            faction_counts=faction_counts,
        )
    
    def _classify_stances(
        self,
        question: str,
        responses: List[AgentResponse]
    ) -> List[AgentResponse]:
        """
        Use LLM to classify stance for each response
        
        Args:
            question: The original question
            responses: List of agent responses
            
        Returns:
            Updated responses with stance classifications
        """
        try:
            # Batch classify stances for efficiency
            batch_size = 10
            all_classified = []
            
            for i in range(0, len(responses), batch_size):
                batch = responses[i:i + batch_size]
                classified = self._classify_batch(question, batch)
                all_classified.extend(classified)
            
            return all_classified
            
        except Exception as e:
            logger.warning(f"Stance classification failed: {e}, using rule-based fallback")
            return self._classify_stances_rule_based(question, responses)
    
    def _classify_batch(
        self,
        question: str,
        responses: List[AgentResponse]
    ) -> List[AgentResponse]:
        """Classify a batch of responses using LLM"""
        
        # Build prompt
        response_texts = []
        for i, r in enumerate(responses):
            response_texts.append(f"{i+1}. {r.agent_name}: {r.response[:300]}")
        
        prompt = f"""Classify each response's stance on the following question.

Question: {question}

Responses:
{chr(10).join(response_texts)}

For each response, classify as:
- "support" if they agree/support the topic
- "oppose" if they disagree/oppose the topic
- "neutral" if they are undecided or balanced

Respond in JSON format:
{{"classifications": [{{"index": 1, "stance": "support"}}, {{"index": 2, "stance": "oppose"}}, ...]}}
"""
        
        try:
            result = self.llm.chat.completions.create(
                model=self.model_name,
                messages=[
                    {"role": "system", "content": "You are a stance classifier. Respond only with valid JSON."},
                    {"role": "user", "content": prompt}
                ],
                response_format={"type": "json_object"},
                temperature=0.3,
                max_tokens=500
            )
            
            content = result.choices[0].message.content
            data = json.loads(content)
            
            # Apply classifications
            for classification in data.get("classifications", []):
                idx = classification.get("index", 0) - 1
                stance_str = classification.get("stance", "neutral").lower()
                
                if 0 <= idx < len(responses):
                    if stance_str == "support":
                        responses[idx].stance = Stance.SUPPORT
                    elif stance_str == "oppose":
                        responses[idx].stance = Stance.OPPOSE
                    else:
                        responses[idx].stance = Stance.NEUTRAL
            
        except Exception as e:
            logger.warning(f"Batch classification failed: {e}")
            # Fall back to rule-based for this batch
            responses = self._classify_stances_rule_based(question, responses)
        
        return responses
    
    def _classify_stances_rule_based(
        self,
        question: str,
        responses: List[AgentResponse]
    ) -> List[AgentResponse]:
        """
        Simple rule-based stance classification as fallback
        """
        support_keywords = [
            "agree", "support", "favor", "approve", "believe", "should",
            "good idea", "positive", "beneficial", "important", "necessary"
        ]
        oppose_keywords = [
            "disagree", "oppose", "against", "reject", "should not",
            "bad idea", "negative", "harmful", "unnecessary", "concerned"
        ]
        
        for r in responses:
            text = r.response.lower()
            
            support_score = sum(1 for kw in support_keywords if kw in text)
            oppose_score = sum(1 for kw in oppose_keywords if kw in text)
            
            if support_score > oppose_score:
                r.stance = Stance.SUPPORT
            elif oppose_score > support_score:
                r.stance = Stance.OPPOSE
            else:
                r.stance = Stance.NEUTRAL
        
        return responses
    
    def generate_summary(
        self,
        result: PanelChatResult
    ) -> str:
        """
        Generate a natural language summary of the panel chat results
        
        Args:
            result: The aggregated panel chat result
            
        Returns:
            Summary string
        """
        try:
            # Build summary with key stats
            support_pct = result.stance_distribution.get("support", 0)
            oppose_pct = result.stance_distribution.get("oppose", 0)
            neutral_pct = result.stance_distribution.get("neutral", 0)
            
            # Get representative quotes
            support_quotes = [r.response[:100] for r in result.by_stance.get("support", [])[:2]]
            oppose_quotes = [r.response[:100] for r in result.by_stance.get("oppose", [])[:2]]
            
            prompt = f"""Summarize this panel discussion in 2-3 sentences:

Question: {result.question}
Total respondents: {result.total_agents}
Support: {support_pct}%
Oppose: {oppose_pct}%
Neutral: {neutral_pct}%

Sample supporting view: {support_quotes[0] if support_quotes else 'None'}
Sample opposing view: {oppose_quotes[0] if oppose_quotes else 'None'}

Provide a balanced summary of the overall sentiment and key tensions."""

            response = self.llm.chat.completions.create(
                model=self.model_name,
                messages=[
                    {"role": "system", "content": "You are a neutral summarizer."},
                    {"role": "user", "content": prompt}
                ],
                temperature=0.5,
                max_tokens=200
            )
            
            return response.choices[0].message.content.strip()
            
        except Exception as e:
            logger.warning(f"Summary generation failed: {e}")
            # Return basic summary
            return (
                f"Of {result.total_agents} respondents, "
                f"{result.stance_distribution.get('support', 0)}% support, "
                f"{result.stance_distribution.get('oppose', 0)}% oppose, and "
                f"{result.stance_distribution.get('neutral', 0)}% are neutral."
            )
