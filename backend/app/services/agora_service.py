"""
Agora Service
Structured debate arena for agent turn-based debates with stance tracking

Provides:
1. Debate state management
2. Turn-based debate execution
3. Stance tracking per round
4. Pause/Resume/Stop controls
"""

import json
import os
import uuid
from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum

from ..config import Config
from ..utils.logger import get_logger
from ..utils.llm_client import LLMClient
from .simulation_runner import SimulationRunner
from .simulation_manager import SimulationManager
from .panel_chat_service import PanelChatService, Stance

logger = get_logger('pubop.agora')


class DebateStatus(Enum):
    """Debate status"""
    CREATED = "created"
    RUNNING = "running"
    PAUSED = "paused"
    COMPLETED = "completed"
    STOPPED = "stopped"
    FAILED = "failed"


class DebateGoal(Enum):
    """Debate goal types (maps to templates internally)"""
    STRESS_TEST = "stress_test"           # Point-Counterpoint
    RISK_IDENTIFICATION = "risk_id"       # Devil's Advocate
    STAKEHOLDER_ANALYSIS = "stakeholder"  # Stakeholder Caucus
    COMPETITIVE_SIM = "competitive"       # Red Team / Blue Team
    FIND_MIDDLE_GROUND = "consensus"      # Consensus Building
    EXPOSE_ASSUMPTIONS = "socratic"       # Socratic Drill


class ModeratorMode(Enum):
    """Moderator mode options"""
    USER_ONLY = "user_only"           # User injects pivots manually
    SYNTHESIZED = "synthesized"       # AI-generated neutral moderator
    FORCED_NEUTRAL = "forced_neutral" # Existing agent forced neutral


class DebateMode(Enum):
    """Debate execution mode"""
    CONTINUOUS = "continuous"   # Auto-run all rounds
    REVIEW = "review"           # Pause after each round for review


@dataclass
class DebateTurn:
    """A single turn in the debate"""
    turn_id: str
    round_num: int
    agent_id: int
    agent_name: str
    response: str
    stance_score: float = 0.0  # -100 to +100
    stance_label: str = "neutral"
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "turn_id": self.turn_id,
            "round_num": self.round_num,
            "agent_id": self.agent_id,
            "agent_name": self.agent_name,
            "response": self.response,
            "stance_score": self.stance_score,
            "stance_label": self.stance_label,
            "timestamp": self.timestamp
        }
    
    @classmethod
    def from_dict(cls, data: Dict) -> 'DebateTurn':
        return cls(
            turn_id=data.get("turn_id", str(uuid.uuid4())[:8]),
            round_num=data.get("round_num", 0),
            agent_id=data.get("agent_id", 0),
            agent_name=data.get("agent_name", ""),
            response=data.get("response", ""),
            stance_score=data.get("stance_score", 0.0),
            stance_label=data.get("stance_label", "neutral"),
            timestamp=data.get("timestamp", datetime.now().isoformat())
        )


@dataclass
class DebateState:
    """Complete debate state"""
    debate_id: str
    simulation_id: str
    topic: str
    goal_type: str
    debate_mode: str = "continuous"
    moderator_mode: str = "user_only"
    
    # Participating agents
    agent_ids: List[int] = field(default_factory=list)
    agent_names: Dict[int, str] = field(default_factory=dict)
    
    # Round configuration
    max_rounds: int = 5
    current_round: int = 0
    turn_timeout: float = 60.0  # Timeout per turn in seconds
    
    # Status
    status: DebateStatus = DebateStatus.CREATED
    
    # Debate content
    turns: List[DebateTurn] = field(default_factory=list)
    moderator_pivots: List[Dict[str, Any]] = field(default_factory=list)
    
    # Stance tracking per agent per round: {agent_id: [(round, score, label), ...]}
    stance_history: Dict[int, List[tuple]] = field(default_factory=dict)
    
    # Summary (generated on stop/complete)
    summary: str = ""
    
    # V2: Timed round configuration
    round_duration_seconds: int = 30
    max_exchanges_per_round: int = 10
    exchanges_per_round: Dict[int, int] = field(default_factory=dict)
    round_summaries: List[Dict[str, Any]] = field(default_factory=list)
    final_summary: Optional[Dict[str, Any]] = None
    
    # Timestamps
    created_at: str = field(default_factory=lambda: datetime.now().isoformat())
    updated_at: str = field(default_factory=lambda: datetime.now().isoformat())
    completed_at: Optional[str] = None
    
    # Error tracking
    error: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "debate_id": self.debate_id,
            "simulation_id": self.simulation_id,
            "topic": self.topic,
            "goal_type": self.goal_type,
            "debate_mode": self.debate_mode,
            "moderator_mode": self.moderator_mode,
            "agent_ids": self.agent_ids,
            "agent_names": self.agent_names,
            "max_rounds": self.max_rounds,
            "current_round": self.current_round,
            "turn_timeout": self.turn_timeout,
            "status": self.status.value if isinstance(self.status, DebateStatus) else self.status,
            "turns": [t.to_dict() for t in self.turns],
            "moderator_pivots": self.moderator_pivots,
            "stance_history": {
                str(k): v for k, v in self.stance_history.items()
            },
            "summary": self.summary,
            "round_duration_seconds": self.round_duration_seconds,
            "max_exchanges_per_round": self.max_exchanges_per_round,
            "exchanges_per_round": self.exchanges_per_round,
            "round_summaries": self.round_summaries,
            "final_summary": self.final_summary,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "completed_at": self.completed_at,
            "error": self.error
        }
    
    @classmethod
    def from_dict(cls, data: Dict) -> 'DebateState':
        turns = [DebateTurn.from_dict(t) for t in data.get("turns", [])]
        status_val = data.get("status", "created")
        status = DebateStatus(status_val) if isinstance(status_val, str) else status_val
        stance_history = {
            int(k): v for k, v in data.get("stance_history", {}).items()
        }
        return cls(
            debate_id=data.get("debate_id", f"debate_{uuid.uuid4().hex[:8]}"),
            simulation_id=data.get("simulation_id", ""),
            topic=data.get("topic", ""),
            goal_type=data.get("goal_type", "stress_test"),
            debate_mode=data.get("debate_mode", "continuous"),
            moderator_mode=data.get("moderator_mode", "user_only"),
            agent_ids=data.get("agent_ids", []),
            agent_names=data.get("agent_names", {}),
            max_rounds=data.get("max_rounds", 5),
            current_round=data.get("current_round", 0),
            turn_timeout=data.get("turn_timeout", 60.0),
            status=status,
            turns=turns,
            moderator_pivots=data.get("moderator_pivots", []),
            stance_history=stance_history,
            summary=data.get("summary", ""),
            round_duration_seconds=data.get("round_duration_seconds", 30),
            max_exchanges_per_round=data.get("max_exchanges_per_round", 10),
            exchanges_per_round=data.get("exchanges_per_round", {}),
            round_summaries=data.get("round_summaries", []),
            final_summary=data.get("final_summary"),
            created_at=data.get("created_at", datetime.now().isoformat()),
            updated_at=data.get("updated_at", datetime.now().isoformat()),
            completed_at=data.get("completed_at"),
            error=data.get("error")
        )


# Goal-to-prompt template mapping
DEBATE_TEMPLATES = {
    "stress_test": {
        "name": "Point-Counterpoint",
        "system_context": "This is a structured debate. Make strong arguments for your position and directly address opposing points.",
        "turn_prompt": "Previous argument: {prev_response}\n\nProvide your rebuttal or counter-argument on the topic: {topic}"
    },
    "risk_id": {
        "name": "Devil's Advocate",
        "system_context": "You must find weaknesses and risks in the proposal. Be critical and thorough.",
        "turn_prompt": "Proposal: {topic}\n\nPrevious point: {prev_response}\n\nIdentify risks, weaknesses, or potential problems with this proposal."
    },
    "stakeholder": {
        "name": "Stakeholder Caucus",
        "system_context": "Represent your stakeholder group's interests. Consider how this affects your constituency.",
        "turn_prompt": "Topic: {topic}\n\nOther stakeholder's view: {prev_response}\n\nRespond from your stakeholder perspective."
    },
    "competitive": {
        "name": "Red Team / Blue Team",
        "system_context": "This is a competitive analysis simulation. Attack or defend based on your assigned role.",
        "turn_prompt": "Scenario: {topic}\n\nOpponent's move: {prev_response}\n\nProvide your competitive response."
    },
    "consensus": {
        "name": "Consensus Building",
        "system_context": "Work toward finding common ground while acknowledging differences.",
        "turn_prompt": "Topic: {topic}\n\nPrevious position: {prev_response}\n\nFind areas of agreement and propose compromise positions."
    },
    "socratic": {
        "name": "Socratic Drill",
        "system_context": "Ask probing questions to expose underlying assumptions and beliefs.",
        "turn_prompt": "Topic: {topic}\n\nPrevious statement: {prev_response}\n\nAsk clarifying questions or challenge the underlying assumptions."
    }
}


class AgoraService:
    """
    Agora Service for structured agent debates
    
    Features:
    - Goal-based debate templates
    - Turn-based execution using existing interview_agent()
    - Stance tracking per round
    - Pause/Resume/Stop controls
    """
    
    def __init__(self):
        self.debates_dir = os.path.join(Config.UPLOAD_FOLDER, "agora")
        os.makedirs(self.debates_dir, exist_ok=True)
        self.panel_chat_service = PanelChatService()
        self._profiles_cache = {}  # Cache loaded profiles per simulation
        self._llm_client = None
    
    @property
    def llm(self) -> LLMClient:
        """Lazy-load LLM client"""
        if self._llm_client is None:
            self._llm_client = LLMClient()
        return self._llm_client
    
    def _load_profiles(self, simulation_id: str) -> Dict[int, Dict]:
        """Load agent profiles for a simulation, returns {agent_id: profile_dict}"""
        if simulation_id in self._profiles_cache:
            return self._profiles_cache[simulation_id]
        
        try:
            manager = SimulationManager()
            profiles = manager.get_profiles(simulation_id, platform="reddit")
            
            # Build lookup by index (agent_id corresponds to index in profiles array)
            profile_lookup = {}
            for idx, profile in enumerate(profiles):
                profile_lookup[idx] = profile
            
            self._profiles_cache[simulation_id] = profile_lookup
            logger.info(f"Loaded {len(profile_lookup)} profiles for simulation {simulation_id}")
            return profile_lookup
            
        except Exception as e:
            logger.error(f"Failed to load profiles for {simulation_id}: {e}")
            return {}
    
    def _generate_profile_response(
        self, 
        profile: Dict, 
        system_context: str, 
        debate_prompt: str,
        agent_name: str
    ) -> str:
        """
        Generate a debate response using agent profile + LLM
        
        Args:
            profile: Agent profile dict with personality, beliefs, bio, etc.
            system_context: Debate template system context
            debate_prompt: The debate turn prompt
            agent_name: Display name for the agent
            
        Returns:
            Generated response text
        """
        # Build a rich profile context
        profile_context = f"""You are {agent_name}.

BACKGROUND:
{profile.get('bio', 'A knowledgeable individual.')}

PERSONALITY:
{profile.get('personality', 'Thoughtful and articulate.')}

POLITICAL VIEWS:
{profile.get('political_views', 'Moderate and pragmatic.')}

OCCUPATION:
{profile.get('occupation', 'Professional')}

BELIEFS AND VALUES:
{profile.get('values', 'Practical and fair-minded.')}

COMMUNICATION STYLE:
- Speak naturally as this person would
- Use their vocabulary and tone
- Reference their background when relevant
- Be authentic to their personality"""

        system_message = f"{profile_context}\n\n{system_context}"
        
        messages = [
            {"role": "system", "content": system_message},
            {"role": "user", "content": debate_prompt}
        ]
        
        try:
            response = self.llm.chat(
                messages=messages,
                temperature=0.8,  # Higher for more natural variation
                max_tokens=1500  # V2: Room for complete substantive responses
            )
            return response.strip()
        except Exception as e:
            logger.error(f"LLM generation failed for {agent_name}: {e}")
            return f"[{agent_name} could not respond due to an error]"

    
    def create_debate(
        self,
        simulation_id: str,
        topic: str,
        goal_type: str,
        agent_ids: List[int],
        agent_names: Dict[int, str],
        max_rounds: int = 5,
        debate_mode: str = "continuous",
        moderator_mode: str = "user_only",
        turn_timeout: float = 60.0,
        round_duration_seconds: int = 30
    ) -> DebateState:
        """
        Create a new debate
        
        Args:
            simulation_id: Simulation ID
            topic: Debate topic
            goal_type: Goal type (stress_test, risk_id, etc.)
            agent_ids: List of participating agent IDs
            agent_names: Mapping of agent_id to name
            max_rounds: Maximum debate rounds
            debate_mode: continuous or review
            moderator_mode: user_only, synthesized, or forced_neutral
            round_duration_seconds: V2 - Duration for each timed round (seconds)
            
        Returns:
            DebateState
        """
        if not topic or not topic.strip():
            raise ValueError("Debate topic is required")
        
        if len(agent_ids) < 2:
            raise ValueError("At least 2 agents required for debate")
        
        if goal_type not in DEBATE_TEMPLATES:
            raise ValueError(f"Invalid goal_type: {goal_type}")
        
        debate_id = f"debate_{uuid.uuid4().hex[:8]}"
        
        state = DebateState(
            debate_id=debate_id,
            simulation_id=simulation_id,
            topic=topic.strip(),
            goal_type=goal_type,
            debate_mode=debate_mode,
            moderator_mode=moderator_mode,
            agent_ids=agent_ids,
            agent_names=agent_names,
            max_rounds=max_rounds,
            turn_timeout=turn_timeout,
            round_duration_seconds=round_duration_seconds,
            status=DebateStatus.CREATED
        )
        
        # Initialize stance history for each agent
        for agent_id in agent_ids:
            state.stance_history[agent_id] = []
        
        self._save_debate(state)
        logger.info(f"Created debate: {debate_id} with {len(agent_ids)} agents, round_duration={round_duration_seconds}s")
        
        return state

    
    def get_debate(self, debate_id: str) -> Optional[DebateState]:
        """Get debate state by ID"""
        debate_path = os.path.join(self.debates_dir, f"{debate_id}.json")
        if not os.path.exists(debate_path):
            return None
        
        with open(debate_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        return DebateState.from_dict(data)
    
    def list_debates(self, simulation_id: str) -> List[Dict]:
        """List all debates for a simulation"""
        debates = []
        
        if not os.path.exists(self.debates_dir):
            return debates
        
        for filename in os.listdir(self.debates_dir):
            if filename.endswith('.json'):
                debate_path = os.path.join(self.debates_dir, filename)
                try:
                    with open(debate_path, 'r', encoding='utf-8') as f:
                        data = json.load(f)
                    
                    # Only include debates for this simulation
                    if data.get('simulation_id') == simulation_id:
                        debates.append({
                            'debate_id': data.get('debate_id'),
                            'topic': data.get('topic'),
                            'status': data.get('status'),
                            'goal_type': data.get('goal_type'),
                            'current_round': data.get('current_round', 0),
                            'max_rounds': data.get('max_rounds', 5),
                            'turn_count': len(data.get('turns', [])),
                            'agent_names': data.get('agent_names', {}),
                            'created_at': data.get('created_at'),
                            'updated_at': data.get('updated_at')
                        })
                except Exception as e:
                    logger.warning(f"Failed to load debate {filename}: {e}")
        
        # Sort by updated_at (most recent first)
        debates.sort(key=lambda x: x.get('updated_at', ''), reverse=True)
        return debates
    
    def execute_round(
        self,
        debate_id: str,
        pivot_topic: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Execute a single debate round
        
        Args:
            debate_id: Debate ID
            pivot_topic: Optional moderator pivot to inject
            
        Returns:
            Round result with turns
        """
        state = self.get_debate(debate_id)
        if not state:
            raise ValueError(f"Debate not found: {debate_id}")
        
        if state.status not in [DebateStatus.CREATED, DebateStatus.RUNNING, DebateStatus.PAUSED]:
            raise ValueError(f"Cannot execute round: debate status is {state.status.value}")
        
        # Update status
        state.status = DebateStatus.RUNNING
        state.current_round += 1
        state.updated_at = datetime.now().isoformat()
        
        # Record pivot if provided
        if pivot_topic:
            state.moderator_pivots.append({
                "round": state.current_round,
                "topic": pivot_topic,
                "timestamp": datetime.now().isoformat()
            })
        
        # Get template
        template = DEBATE_TEMPLATES.get(state.goal_type, DEBATE_TEMPLATES["stress_test"])
        
        # Get last response for context
        prev_response = ""
        if state.turns:
            prev_response = state.turns[-1].response
        
        # Use pivot topic if provided, otherwise original topic
        current_topic = pivot_topic if pivot_topic else state.topic
        
        round_turns = []
        
        # Load profiles for this simulation (Option 2: profile-based debates)
        profiles = self._load_profiles(state.simulation_id)
        
        # Execute turn for each agent
        for agent_id in state.agent_ids:
            try:
                # Build prompt
                prompt = template["turn_prompt"].format(
                    topic=current_topic,
                    prev_response=prev_response if prev_response else "(Opening statement)"
                )
                
                # Get agent profile
                profile = profiles.get(agent_id, {})
                agent_name = state.agent_names.get(agent_id, f"Agent_{agent_id}")
                
                # Generate response using profile + LLM (Option 2)
                response_text = self._generate_profile_response(
                    profile=profile,
                    system_context=template["system_context"],
                    debate_prompt=prompt,
                    agent_name=agent_name
                )
                
                if response_text and not response_text.startswith("["):
                    # Classify stance
                    stance_label, stance_score = self._classify_stance(state.topic, response_text)
                    
                    turn = DebateTurn(
                        turn_id=f"t{state.current_round}_{agent_id}",
                        round_num=state.current_round,
                        agent_id=agent_id,
                        agent_name=agent_name,
                        response=response_text,
                        stance_score=stance_score,
                        stance_label=stance_label
                    )
                    
                    state.turns.append(turn)
                    round_turns.append(turn)
                    
                    # Update stance history
                    if agent_id not in state.stance_history:
                        state.stance_history[agent_id] = []
                    state.stance_history[agent_id].append(
                        (state.current_round, stance_score, stance_label)
                    )
                    
                    # Update prev_response for next agent
                    prev_response = response_text
                    
                    logger.info(f"Agent {agent_name} responded with {len(response_text)} chars")
                else:
                    logger.warning(f"Agent {agent_id} response generation failed or empty")
                    
            except Exception as e:
                logger.error(f"Error executing turn for agent {agent_id}: {e}")
        
        # Check if debate complete
        if state.current_round >= state.max_rounds:
            state.status = DebateStatus.COMPLETED
            state.completed_at = datetime.now().isoformat()
        elif state.debate_mode == "review":
            state.status = DebateStatus.PAUSED
        
        self._save_debate(state)
        
        return {
            "debate_id": debate_id,
            "round": state.current_round,
            "turns": [t.to_dict() for t in round_turns],
            "status": state.status.value,
            "is_complete": state.status == DebateStatus.COMPLETED
        }
    
    def execute_timed_round(
        self,
        debate_id: str,
        round_duration_seconds: Optional[int] = None,
        pivot_topic: Optional[str] = None
    ):
        """
        Execute a timed round with continuous A→B→A→B exchanges.
        
        This is a generator that yields turn objects as they complete,
        allowing real-time streaming to the frontend.
        
        Args:
            debate_id: Debate ID
            round_duration_seconds: Duration for this round (uses state default if None)
            pivot_topic: Optional moderator pivot to inject
            
        Yields:
            Turn dicts as they complete
        """
        import time
        
        state = self.get_debate(debate_id)
        if not state:
            raise ValueError(f"Debate not found: {debate_id}")
        
        if state.status not in [DebateStatus.CREATED, DebateStatus.RUNNING, DebateStatus.PAUSED]:
            raise ValueError(f"Cannot execute round: debate status is {state.status.value}")
        
        # Update status
        state.status = DebateStatus.RUNNING
        state.current_round += 1
        state.updated_at = datetime.now().isoformat()
        
        # Use provided duration or state default
        duration = round_duration_seconds or state.round_duration_seconds
        max_exchanges = state.max_exchanges_per_round
        
        # Record pivot if provided
        if pivot_topic:
            state.moderator_pivots.append({
                "round": state.current_round,
                "topic": pivot_topic,
                "timestamp": datetime.now().isoformat()
            })
        
        # Get template
        template = DEBATE_TEMPLATES.get(state.goal_type, DEBATE_TEMPLATES["stress_test"])
        
        # Use pivot topic if provided, otherwise original topic
        current_topic = pivot_topic if pivot_topic else state.topic
        
        # Load profiles
        profiles = self._load_profiles(state.simulation_id)
        
        # Build conversation history for context
        conversation_history = []
        for turn in state.turns[-6:]:  # Last 6 turns for context
            conversation_history.append(f"{turn.agent_name}: {turn.response}")
        
        round_start = time.time()
        exchange_count = 0
        current_agent_idx = 0
        
        logger.info(f"Starting timed round {state.current_round} for {duration}s (max {max_exchanges} exchanges)")
        
        # A→B→A→B pattern until time runs out or max exchanges reached
        while True:
            elapsed = time.time() - round_start
            
            # Check termination conditions
            if elapsed >= duration:
                logger.info(f"Round {state.current_round} ended: time limit reached ({elapsed:.1f}s)")
                break
            
            if exchange_count >= max_exchanges:
                logger.info(f"Round {state.current_round} ended: max exchanges reached ({exchange_count})")
                break
            
            # Get current agent (alternating A→B→A→B)
            agent_id = state.agent_ids[current_agent_idx % len(state.agent_ids)]
            # Try both int and string keys (JSON serialization converts int keys to strings)
            agent_name = state.agent_names.get(agent_id) or state.agent_names.get(str(agent_id)) or f"Agent_{agent_id}"
            logger.info(f"[Agora] Agent lookup: id={agent_id}, name={agent_name}, names_dict={state.agent_names}")
            
            # Build context-aware prompt with conversation history
            history_text = "\n".join(conversation_history[-4:]) if conversation_history else ""
            
            # V2: Conversational 3-sentence prompt (no topic introductions, natural flow)
            if conversation_history:
                prompt = f"""You are in a live debate about: {current_topic}

The conversation so far:
{history_text}

Continue the debate naturally. Respond DIRECTLY to what was just said - agree, disagree, or challenge their point. Speak like a real person in an argument.

RULES:
- 3-4 sentences with substance
- NO topic introductions ("Well, regarding X...")
- Give REASONS for your position (because X, which leads to Y)
- Include a concrete example or consequence
- Reference what the other person said and challenge it
- Be direct - this is a real debate"""
            else:
                prompt = f"""You are starting a debate about: {current_topic}

Make your opening statement. Take a clear, arguable position.

RULES:
- 3-4 sentences with substance
- NO greetings or "I believe that..." openings
- Give your strongest argument WITH reasoning (because X leads to Y)
- Include a concrete example or potential consequence
- Be provocative to invite strong rebuttal"""
            
            try:
                # Get agent profile
                profile = profiles.get(agent_id, {})
                
                # Generate response
                response_text = self._generate_profile_response(
                    profile=profile,
                    system_context=template["system_context"],
                    debate_prompt=prompt,
                    agent_name=agent_name
                )
                
                if response_text and not response_text.startswith("["):
                    # Classify stance
                    stance_label, stance_score = self._classify_stance(state.topic, response_text)
                    
                    turn = DebateTurn(
                        turn_id=f"t{state.current_round}_{exchange_count}_{agent_id}",
                        round_num=state.current_round,
                        agent_id=agent_id,
                        agent_name=agent_name,
                        response=response_text,
                        stance_score=stance_score,
                        stance_label=stance_label
                    )
                    
                    state.turns.append(turn)
                    
                    # Update conversation history
                    conversation_history.append(f"{agent_name}: {response_text}")
                    
                    # Update stance history
                    if agent_id not in state.stance_history:
                        state.stance_history[agent_id] = []
                    state.stance_history[agent_id].append(
                        (state.current_round, stance_score, stance_label)
                    )
                    
                    exchange_count += 1
                    
                    logger.info(f"[R{state.current_round}] {agent_name}: {len(response_text)} chars ({elapsed:.1f}s)")
                    
                    # Save incrementally
                    self._save_debate(state)
                    
                    # Yield turn for streaming
                    yield turn.to_dict()
                    
            except Exception as e:
                logger.error(f"Error in timed round for agent {agent_id}: {e}")
            
            # Move to next agent
            current_agent_idx += 1
        
        # Record exchanges for this round
        state.exchanges_per_round[state.current_round] = exchange_count
        
        # Check if debate complete
        if state.current_round >= state.max_rounds:
            state.status = DebateStatus.COMPLETED
            state.completed_at = datetime.now().isoformat()
        elif state.debate_mode == "review":
            state.status = DebateStatus.PAUSED
        
        self._save_debate(state)
        
        # Yield final round metadata
        yield {
            "_type": "round_complete",
            "round": state.current_round,
            "exchanges": exchange_count,
            "duration_seconds": time.time() - round_start,
            "status": state.status.value,
            "is_complete": state.status == DebateStatus.COMPLETED
        }
    
    def pause_debate(self, debate_id: str) -> DebateState:
        """Pause a running debate"""
        state = self.get_debate(debate_id)
        if not state:
            raise ValueError(f"Debate not found: {debate_id}")
        
        if state.status != DebateStatus.RUNNING:
            raise ValueError(f"Cannot pause: debate status is {state.status.value}")
        
        state.status = DebateStatus.PAUSED
        state.updated_at = datetime.now().isoformat()
        self._save_debate(state)
        
        logger.info(f"Paused debate: {debate_id}")
        return state
    
    def resume_debate(self, debate_id: str) -> DebateState:
        """Resume a paused debate"""
        state = self.get_debate(debate_id)
        if not state:
            raise ValueError(f"Debate not found: {debate_id}")
        
        if state.status != DebateStatus.PAUSED:
            raise ValueError(f"Cannot resume: debate status is {state.status.value}")
        
        state.status = DebateStatus.RUNNING
        state.updated_at = datetime.now().isoformat()
        self._save_debate(state)
        
        logger.info(f"Resumed debate: {debate_id}")
        return state
    
    def stop_debate(self, debate_id: str, generate_summary: bool = True) -> DebateState:
        """
        Stop a debate permanently and generate summary
        
        Args:
            debate_id: Debate ID
            generate_summary: Whether to generate summary
            
        Returns:
            Final DebateState
        """
        state = self.get_debate(debate_id)
        if not state:
            raise ValueError(f"Debate not found: {debate_id}")
        
        if state.status in [DebateStatus.COMPLETED, DebateStatus.STOPPED]:
            return state  # Already finished
        
        state.status = DebateStatus.STOPPED
        state.completed_at = datetime.now().isoformat()
        state.updated_at = datetime.now().isoformat()
        
        if generate_summary and state.turns:
            state.summary = self._generate_summary(state)
        
        self._save_debate(state)
        
        logger.info(f"Stopped debate: {debate_id}")
        return state
    
    def _extract_response_text(self, result: Dict) -> str:
        """Extract response text from interview result"""
        if isinstance(result, str):
            return result
        
        # Handle dual-platform response
        if "platforms" in result:
            platforms = result.get("platforms", {})
            for platform in ["twitter", "reddit"]:
                if platform in platforms:
                    return platforms[platform].get("response", "")
        
        # Handle single platform response
        return result.get("response", str(result))
    
    def _classify_stance(self, topic: str, response: str) -> tuple:
        """
        Classify stance from response
        
        Returns:
            (stance_label, stance_score)
        """
        # Use simple keyword-based classification for now
        # Can be enhanced with PanelChatService._classify_stances() later
        response_lower = response.lower()
        
        support_keywords = ["agree", "support", "favor", "approve", "yes", "correct", "valid"]
        oppose_keywords = ["disagree", "oppose", "against", "reject", "no", "wrong", "invalid"]
        
        support_count = sum(1 for kw in support_keywords if kw in response_lower)
        oppose_count = sum(1 for kw in oppose_keywords if kw in response_lower)
        
        if support_count > oppose_count:
            score = min(100, support_count * 25)
            return "support", score
        elif oppose_count > support_count:
            score = max(-100, -oppose_count * 25)
            return "oppose", score
        else:
            return "neutral", 0
    
    def _generate_summary(self, state: DebateState) -> str:
        """Generate a proper synopsis summary of the debate using LLM"""
        if not state.turns:
            return "No debate turns recorded."
        
        # Build basic stats
        template_name = DEBATE_TEMPLATES.get(state.goal_type, {}).get("name", "Debate")
        
        # Count stances
        stance_counts = {"support": 0, "oppose": 0, "neutral": 0}
        for turn in state.turns:
            label = turn.stance_label.lower()
            if label in stance_counts:
                stance_counts[label] += 1
        
        total_turns = len(state.turns)
        
        # Get participant names (actual names, not Agent_X)
        participant_names = list(state.agent_names.values())
        
        # Determine overall lean
        if stance_counts['support'] > stance_counts['oppose']:
            overall_lean = "leaning toward support"
        elif stance_counts['oppose'] > stance_counts['support']:
            overall_lean = "leaning toward opposition"
        else:
            overall_lean = "balanced/contested"
        
        # Build debate transcript for LLM
        transcript_for_llm = []
        for turn in state.turns:
            name = state.agent_names.get(turn.agent_id, turn.agent_name)
            transcript_for_llm.append(f"{name}: {turn.response}")
        
        # Use LLM to generate synopsis
        try:
            synopsis_prompt = f"""You are summarizing a debate for a decision-maker.

TOPIC: {state.topic}
PARTICIPANTS: {', '.join(participant_names)}
TOTAL EXCHANGES: {total_turns}
OVERALL LEAN: {overall_lean}

DEBATE TRANSCRIPT:
{chr(10).join(transcript_for_llm[-10:])}

Write a concise 3-paragraph summary:

PARAGRAPH 1 - DIRECTION: What was the overall direction of the debate? Did positions shift or harden?

PARAGRAPH 2 - KEY ARGUMENTS: What were the 2-3 most compelling arguments made by each side? Reference participants BY NAME.

PARAGRAPH 3 - IMPLICATIONS: What are the key takeaways for decision-making? What risks or opportunities emerged?

Use actual participant names ({', '.join(participant_names)}), NOT "Agent_0" or similar.
Be analytical, not just descriptive. This summary should provide actionable insight."""

            synopsis = self.llm.chat(
                messages=[
                    {"role": "system", "content": "You are an expert debate analyst. Summarize debates with precision and insight."},
                    {"role": "user", "content": synopsis_prompt}
                ],
                temperature=0.3,
                max_tokens=800
            )
        except Exception as e:
            logger.error(f"LLM synopsis generation failed: {e}")
            synopsis = f"Debate covered {total_turns} exchanges on the topic. Participants were {overall_lean}."
        
        # Build formatted summary
        summary_parts = [
            f"## {template_name} Summary",
            f"",
            f"**Topic:** {state.topic}",
            f"",
            f"**Participants:** {', '.join(participant_names)}",
            f"",
            f"**Rounds:** {state.current_round}/{state.max_rounds} | **Exchanges:** {total_turns}",
            f"",
            f"### Stance Distribution",
            f"",
            f"| Position | Count |",
            f"|----------|-------|",
            f"| Support  | {stance_counts['support']} |",
            f"| Oppose   | {stance_counts['oppose']} |",
            f"| Neutral  | {stance_counts['neutral']} |",
            f"",
            f"**Overall Lean:** {overall_lean.title()}",
            f"",
            f"---",
            f"",
            f"### Synopsis",
            f"",
            synopsis.strip()
        ]
        
        return "\n".join(summary_parts)
    
    def _save_debate(self, state: DebateState):
        """Save debate state to file"""
        debate_path = os.path.join(self.debates_dir, f"{state.debate_id}.json")
        with open(debate_path, 'w', encoding='utf-8') as f:
            json.dump(state.to_dict(), f, ensure_ascii=False, indent=2)
