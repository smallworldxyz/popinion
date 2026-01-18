import json
import os
from typing import Dict, List, Any, Optional
from ..utils.logger import get_logger
from .simulation_ipc import SimulationIPCClient, CommandType

logger = get_logger('oasis.scenario')

class ScenarioManager:
    """
    Manages the lifecycle of a scripted scenario: loading, monitoring triggers, and executing actions.
    """
    def __init__(self, simulation_dir: str, scenario_path: Optional[str] = None):
        self.simulation_dir = simulation_dir
        self.ipc_client = SimulationIPCClient(simulation_dir)
        self.scenario_data = None
        self.active_timeline = []
        self.executed_indices = set()
        
        if scenario_path:
            self.load_scenario(scenario_path)

    def load_scenario(self, path: str) -> bool:
        """Load and validate a scenario file."""
        try:
            if not os.path.exists(path):
                logger.error(f"Scenario file not found: {path}")
                return False
                
            with open(path, 'r', encoding='utf-8') as f:
                data = json.load(f)
                
            # Basic validation
            if 'timeline' not in data:
                logger.error("Invalid scenario: missing 'timeline'")
                return False
                
            self.scenario_data = data
            self.active_timeline = sorted(data['timeline'], key=lambda x: x.get('trigger', {}).get('value', 0))
            self.executed_indices = set()
            
            logger.info(f"Loaded scenario: {data.get('meta', {}).get('name')} ({len(self.active_timeline)} events)")
            return True
            
        except Exception as e:
            logger.error(f"Failed to load scenario {path}: {e}")
            return False

    def check_and_execute(self, current_state: Dict[str, Any]):
        """
        Check triggers against current state and execute actions if met.
        
        Args:
            current_state: Dict containing 'current_round', 'simulated_hours', etc.
        """
        if not self.scenario_data:
            return

        current_round = current_state.get('current_round', -1)
        
        for idx, event in enumerate(self.active_timeline):
            if idx in self.executed_indices:
                continue
                
            trigger = event.get('trigger', {})
            t_type = trigger.get('type')
            t_value = trigger.get('value')
            
            # TRIGGER LOGIC
            should_fire = False
            
            if t_type == 'round':
                # Fire exactly at the start of the round
                if current_round == t_value:
                    should_fire = True
            
            # Execute if triggered
            if should_fire:
                logger.info(f"Scenario Trigger Encounters: Round {current_round} -> Executing Event {idx}")
                self._execute_action(event.get('action', {}))
                self.executed_indices.add(idx)

    def _execute_action(self, action: Dict[str, Any]):
        """Execute a single scenario action."""
        a_type = action.get('type')
        payload = action.get('payload', {})
        
        try:
            if a_type == 'inject_event':
                content = payload.get('content')
                source = payload.get('source', 'System')
                if content:
                    full_text = f"[{source}] {content}" # Add source prefix if present
                    logger.info(f"Injecting Scenario Event: {full_text}")
                    self.ipc_client.send_inject_event(full_text)
            else:
                logger.warning(f"Unknown scenario action type: {a_type}")
                
        except Exception as e:
            logger.error(f"Failed to execute scenario action: {e}")
