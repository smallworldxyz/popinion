
"""
Simulation Service Layer
Handles business logic for simulation orchestration, state management, and configuration.
Refactored to use BaseService and Result[T] pattern.
"""

import os
import json
import threading
from datetime import datetime
from typing import Optional, Dict, Any, List
from dataclasses import dataclass

from ..config import Config
from ..services.simulation_manager import SimulationManager, SimulationStatus
from ..services.neo4j_entity_reader import Neo4jEntityReader
from ..services.base_service import BaseService, Result, ErrorCode
from ..models.project import ProjectManager
from ..models.task import TaskManager, TaskStatus
from ..exceptions import NotFoundError, SimulationError


@dataclass
class PrepareCheckInfo:
    """Information about simulation preparation status"""
    is_prepared: bool
    status: str
    entities_count: int = 0
    profiles_count: int = 0
    entity_types: List[str] = None
    config_generated: bool = False
    created_at: Optional[str] = None
    updated_at: Optional[str] = None
    existing_files: List[str] = None
    reason: Optional[str] = None


@dataclass
class PreparationTaskResult:
    """Result of starting a preparation task"""
    task_id: Optional[str]
    status: str
    message: str
    already_prepared: bool
    prepare_information: Optional[Dict[str, Any]] = None


class SimulationService(BaseService):
    """
    Service layer for simulation management.
    Provides clean interface for API routes with consistent error handling.
    """
    
    def __init__(self):
        super().__init__(name="simulation")
    
    def check_prepared(self, simulation_id: str) -> Result[PrepareCheckInfo]:
        """
        Check if simulation is already prepared.
        
        Returns:
            Result containing PrepareCheckInfo or error
        """
        simulation_dir = os.path.join(Config.OASIS_SIMULATION_DATA_DIR, simulation_id)
        
        # Check if directory exists
        if not os.path.exists(simulation_dir):
            return Result.success(PrepareCheckInfo(
                is_prepared=False,
                status="not_initialized",
                reason="Simulation directory does not exist"
            ))
        
        # Required files
        required_files = [
            "state.json",
            "simulation_config.json",
            "reddit_profiles.json",
            "twitter_profiles.csv"
        ]
        
        # Check if files exist
        existing_files = []
        missing_files = []
        for f in required_files:
            file_path = os.path.join(simulation_dir, f)
            if os.path.exists(file_path):
                existing_files.append(f)
            else:
                missing_files.append(f)
        
        if missing_files:
            return Result.success(PrepareCheckInfo(
                is_prepared=False,
                status="incomplete",
                reason=f"Missing required files: {', '.join(missing_files)}",
                existing_files=existing_files
            ))
        
        # Check state.json status
        state_file = os.path.join(simulation_dir, "state.json")
        try:
            with open(state_file, 'r', encoding='utf-8') as f:
                state_data = json.load(f)
            
            status = state_data.get("status", "")
            config_generated = state_data.get("config_generated", False)
            
            prepared_statuses = ["ready", "preparing", "running", "completed", "stopped", "failed"]
            
            if status in prepared_statuses and config_generated:
                # Get file statistics
                profiles_file = os.path.join(simulation_dir, "reddit_profiles.json")
                profiles_count = 0
                if os.path.exists(profiles_file):
                    with open(profiles_file, 'r', encoding='utf-8') as f:
                        profiles_data = json.load(f)
                        profiles_count = len(profiles_data) if isinstance(profiles_data, list) else 0
                
                # Auto-update status if stuck in preparing
                if status == "preparing":
                    try:
                        state_data["status"] = "ready"
                        state_data["updated_at"] = datetime.now().isoformat()
                        with open(state_file, 'w', encoding='utf-8') as f:
                            json.dump(state_data, f, ensure_ascii=False, indent=2)
                        status = "ready"
                    except Exception as e:
                        self.logger.warning(f"Auto-update status failed: {e}")
                
                return Result.success(PrepareCheckInfo(
                    is_prepared=True,
                    status=status,
                    entities_count=state_data.get("entities_count", 0),
                    profiles_count=profiles_count,
                    entity_types=state_data.get("entity_types", []),
                    config_generated=config_generated,
                    created_at=state_data.get("created_at"),
                    updated_at=state_data.get("updated_at"),
                    existing_files=existing_files
                ))
            else:
                return Result.success(PrepareCheckInfo(
                    is_prepared=False,
                    status=status,
                    reason=f"Status not in prepared list or config_generated is false",
                    config_generated=config_generated
                ))
                
        except Exception as e:
            return Result.failure(
                ErrorCode.INTERNAL_ERROR,
                f"Failed to read status file: {str(e)}"
            )

    def create_simulation(
        self, 
        project_id: str, 
        graph_id: str, 
        enable_twitter: bool = True, 
        enable_reddit: bool = True
    ) -> Result[Dict[str, Any]]:
        """
        Create a new simulation record.
        
        Returns:
            Result containing simulation state dict
        """
        try:
            manager = SimulationManager()
            state = manager.create_simulation(
                project_id=project_id,
                graph_id=graph_id,
                enable_twitter=enable_twitter,
                enable_reddit=enable_reddit,
            )
            return Result.success(state.to_dict())
        except Exception as e:
            self.logger.error(f"Failed to create simulation: {e}")
            return Result.failure(
                ErrorCode.INTERNAL_ERROR,
                f"Failed to create simulation: {str(e)}"
            )

    def start_preparation_task(
        self, 
        simulation_id: str, 
        project_id: str, 
        graph_id: str,
        simulation_requirement: str, 
        document_text: str,
        entity_types: List[str] = None, 
        selected_entity_ids: List[str] = None,
        use_llm_for_profiles: bool = True, 
        parallel_profile_count: int = 5,
        force_regenerate: bool = False
    ) -> Result[PreparationTaskResult]:
        """
        Orchestrate the async preparation task.
        
        Returns:
            Result containing PreparationTaskResult
        """
        # Check if prepared (unless force regenerate)
        if not force_regenerate:
            check_result = self.check_prepared(simulation_id)
            if check_result.ok and check_result.value.is_prepared:
                info = check_result.value
                return Result.success(PreparationTaskResult(
                    task_id=None,
                    status="ready",
                    message="Already prepared",
                    already_prepared=True,
                    prepare_information={
                        "status": info.status,
                        "entities_count": info.entities_count,
                        "profiles_count": info.profiles_count
                    }
                ))

        manager = SimulationManager()
        state = manager.get_simulation(simulation_id)
        if not state:
            return Result.failure(
                ErrorCode.NOT_FOUND,
                f"Simulation not found: {simulation_id}"
            )

        # Create Task
        task_manager = TaskManager()
        task_id = task_manager.create_task(
            task_type="simulation_prepare",
            metadata={"simulation_id": simulation_id, "project_id": project_id}
        )
        
        # Update State
        state.status = SimulationStatus.PREPARING
        manager._save_simulation_state(state)
        
        # Define Background Worker
        def run_prepare():
            try:
                task_manager.update_task(task_id, status=TaskStatus.PROCESSING, progress=0, message="Starting preparation...")
                
                def progress_callback(stage, progress, message, **kwargs):
                    msg = f"{stage}: {message}"
                    task_manager.update_task(task_id, progress=progress, message=msg)
                
                result_state = manager.prepare_simulation(
                    simulation_id=simulation_id,
                    simulation_requirement=simulation_requirement,
                    document_text=document_text,
                    defined_entity_types=entity_types,
                    selected_entity_ids=selected_entity_ids,
                    use_llm_for_profiles=use_llm_for_profiles,
                    progress_callback=progress_callback,
                    parallel_profile_count=parallel_profile_count
                )
                
                task_manager.complete_task(task_id, result=result_state.to_simple_dict())
                
            except Exception as e:
                self.logger.error(f"Prepare failed: {e}")
                task_manager.fail_task(task_id, str(e))
                # Update simulation state
                s = manager.get_simulation(simulation_id)
                if s:
                    s.status = SimulationStatus.FAILED
                    s.error = str(e)
                    manager._save_simulation_state(s)

        # Start Thread
        thread = threading.Thread(target=run_prepare, daemon=True)
        thread.start()
        
        return Result.success(PreparationTaskResult(
            task_id=task_id,
            status="preparing",
            message="Preparation task started",
            already_prepared=False
        ))


# Singleton instance for convenience
_service_instance: Optional[SimulationService] = None

def get_simulation_service() -> SimulationService:
    """Get or create singleton SimulationService instance"""
    global _service_instance
    if _service_instance is None:
        _service_instance = SimulationService()
    return _service_instance
