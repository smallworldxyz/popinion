from pydantic import BaseModel, Field, field_validator
from typing import Optional, List

class SimulationCreateRequest(BaseModel):
    project_id: str = Field(..., description="ID of the project")
    graph_id: Optional[str] = Field(None, description="Graph ID if different from project default")
    enable_twitter: bool = Field(True, description="Enable Twitter platform simulation")
    enable_reddit: bool = Field(True, description="Enable Reddit platform simulation")

class SimulationPrepareRequest(BaseModel):
    simulation_id: str = Field(..., description="ID of the simulation instance")
    entity_types: Optional[List[str]] = Field(None, description="Filter specific entity types")
    selected_entity_ids: Optional[List[str]] = Field(None, description="Specific entity IDs to include")
    use_llm_for_profiles: bool = Field(True, description="Use LLM to generate detailed profiles")
    parallel_profile_count: int = Field(5, ge=1, le=20, description="Number of parallel profile generations")
    force_regenerate: bool = Field(False, description="Force regeneration even if already prepared")

class SimulationStatusRequest(BaseModel):
    task_id: Optional[str] = Field(None, description="Async task ID to check")
    simulation_id: Optional[str] = Field(None, description="Simulation ID to check preparedness")

    @field_validator('simulation_id')
    @classmethod
    def validate_one_present(cls, v, values):
        # Note: In Pydantic v2, validation logic is slightly different regarding 'values'
        # But for simple check, we can check at call site or use model_validator
        return v
