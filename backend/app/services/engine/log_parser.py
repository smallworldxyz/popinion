
"""
Simulation Log Parser
Handles reading and parsing of action logs.
"""

import os
import json
from datetime import datetime
from typing import List, Optional, Dict, Any
from .state import AgentAction, SimulationRunState, RunnerStatus
from ...services.neo4j_graph_memory_updater import Neo4jGraphMemoryManager
from ...utils.logger import get_logger

logger = get_logger('pubop.engine.log_parser')

class LogParser:
    
    @staticmethod
    def read_action_log(
        log_path: str, 
        position: int, 
        state: SimulationRunState,
        platform: str,
        enable_graph_memory_update: bool = False
    ) -> int:
        """
        Read action log file and update state
        Returns new position
        """
        graph_updater = None
        if enable_graph_memory_update:
            graph_updater = Neo4jGraphMemoryManager.get_updater(state.simulation_id)
        
        try:
            if not os.path.exists(log_path):
                return position

            with open(log_path, 'r', encoding='utf-8') as f:
                f.seek(position)
                for line in f:
                    line = line.strip()
                    if line:
                        try:
                            action_data = json.loads(line)
                            
                            # Process event type items
                            if "event_type" in action_data:
                                LogParser._process_event(action_data, state, platform)
                                continue
                            
                            # Process normal action
                            LogParser._process_action(action_data, state, platform, graph_updater)
                            
                        except json.JSONDecodeError:
                            pass
                return f.tell()
        except Exception as e:
            logger.warning(f"Failed to read action log: {log_path}, error={e}")
            return position

    @staticmethod
    def _process_event(data: Dict, state: SimulationRunState, platform: str):
        event_type = data.get("event_type")
        
        if event_type == "simulation_end":
            if platform == "twitter":
                state.twitter_completed = True
                state.twitter_running = False
            elif platform == "reddit":
                state.reddit_completed = True
                state.reddit_running = False
            
            # Check all completed
            if LogParser.check_all_platforms_completed(state):
                state.runner_status = RunnerStatus.COMPLETED
                state.completed_at = datetime.now().isoformat()
        
        elif event_type == "round_end":
            round_num = data.get("round", 0)
            simulated_hours = data.get("simulated_hours", 0)
            
            if platform == "twitter":
                if round_num > state.twitter_current_round:
                    state.twitter_current_round = round_num
                state.twitter_simulated_hours = simulated_hours
            elif platform == "reddit":
                if round_num > state.reddit_current_round:
                    state.reddit_current_round = round_num
                state.reddit_simulated_hours = simulated_hours
            
            if round_num > state.current_round:
                state.current_round = round_num
            state.simulated_hours = max(state.twitter_simulated_hours, state.reddit_simulated_hours)

    @staticmethod
    def _process_action(data: Dict, state: SimulationRunState, platform: str, graph_updater):
        action = AgentAction(
            round_num=data.get("round", 0),
            timestamp=data.get("timestamp", datetime.now().isoformat()),
            platform=platform,
            agent_id=data.get("agent_id", 0),
            agent_name=data.get("agent_name", ""),
            action_type=data.get("action_type", ""),
            action_args=data.get("action_args", {}),
            result=data.get("result"),
            success=data.get("success", True),
        )
        state.add_action(action)
        
        if action.round_num and action.round_num > state.current_round:
            state.current_round = action.round_num
        
        if graph_updater:
            graph_updater.add_activity_from_dict(data, platform)

    @staticmethod
    def check_all_platforms_completed(state: SimulationRunState) -> bool:
        # This logic needs access to the file system to know which platforms are enabled
        # Ideally, enabled platforms should be stored in state, but currently we infer from log existence.
        # We will keep the inference logic but move path resolution to a helper or pass it in.
        # For simplicity, we re-implement the path logic here as it depends on directory structure
        
        from .state import StateManager
        sim_dir = os.path.join(StateManager.RUN_STATE_DIR, state.simulation_id)
        twitter_log = os.path.join(sim_dir, "twitter", "actions.jsonl")
        reddit_log = os.path.join(sim_dir, "reddit", "actions.jsonl")
        
        twitter_enabled = os.path.exists(twitter_log)
        reddit_enabled = os.path.exists(reddit_log)
        
        if twitter_enabled and not state.twitter_completed:
            return False
        if reddit_enabled and not state.reddit_completed:
            return False
            
        return twitter_enabled or reddit_enabled

    @staticmethod
    def get_all_actions(sim_dir: str, platform: Optional[str] = None, 
                       agent_id: Optional[int] = None, round_num: Optional[int] = None) -> List[AgentAction]:
        actions = []
        
        # Helper to read file
        def read_file(path, default_platform):
            if not os.path.exists(path): return []
            file_actions = []
            with open(path, 'r', encoding='utf-8') as f:
                for line in f:
                    try:
                        data = json.loads(line)
                        if "event_type" in data or "agent_id" not in data: continue
                        
                        rec_platform = data.get("platform") or default_platform
                        if platform and rec_platform != platform: continue
                        if agent_id is not None and data.get("agent_id") != agent_id: continue
                        if round_num is not None and data.get("round") != round_num: continue
                        
                        file_actions.append(AgentAction(
                            round_num=data.get("round", 0),
                            timestamp=data.get("timestamp", ""),
                            platform=rec_platform,
                            agent_id=data.get("agent_id", 0),
                            agent_name=data.get("agent_name", ""),
                            action_type=data.get("action_type", ""),
                            action_args=data.get("action_args", {}),
                            result=data.get("result"),
                            success=data.get("success", True),
                        ))
                    except: pass
            return file_actions

        if not platform or platform == "twitter":
            actions.extend(read_file(os.path.join(sim_dir, "twitter", "actions.jsonl"), "twitter"))
            
        if not platform or platform == "reddit":
            actions.extend(read_file(os.path.join(sim_dir, "reddit", "actions.jsonl"), "reddit"))
            
        if not actions:
            actions.extend(read_file(os.path.join(sim_dir, "actions.jsonl"), None))
            
        actions.sort(key=lambda x: x.timestamp, reverse=True)
        return actions
