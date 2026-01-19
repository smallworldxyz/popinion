
"""
OASIS Simulation Runner (Legacy Proxy)
Forwards to new Engine subsystem.
"""

# Re-export from new engine
from .engine.runner import SimulationRunner
from .engine.state import RunnerStatus, SimulationRunState, AgentAction, RoundSummary
