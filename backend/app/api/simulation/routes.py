
from flask import Blueprint, request, jsonify
from dataclasses import asdict

from ...services.simulation_service import get_simulation_service
from ...services.simulation_manager import SimulationManager
from ...services.neo4j_entity_reader import Neo4jEntityReader
from ...models.project import ProjectManager
from ...models.task import TaskManager
from ...auth import login_required
from ...utils.logger import get_logger
from ...schemas.simulation import (
    SimulationCreateRequest, 
    SimulationPrepareRequest, 
    SimulationStatusRequest
)
from pydantic import ValidationError

logger = get_logger('pubop.api.simulation.routes')
routes_bp = Blueprint('simulation_routes', __name__)


def result_to_response(result):
    """Convert Result[T] to Flask response"""
    if result.ok:
        value = result.value
        # Convert dataclass to dict if needed
        if hasattr(value, '__dataclass_fields__'):
            data = asdict(value)
        elif isinstance(value, dict):
            data = value
        else:
            data = value
        return jsonify({"success": True, "data": data})
    else:
        # Map error code to HTTP status
        status_map = {
            "not_found": 404,
            "validation_error": 400,
            "internal_error": 500
        }
        status = status_map.get(result.error.code.value, 500)
        return jsonify({
            "success": False, 
            "error": result.error.message,
            "code": result.error.code.value,
            "details": result.error.details
        }), status


@routes_bp.route('/create', methods=['POST'])
@login_required
def create_simulation():
    try:
        req = SimulationCreateRequest(**(request.get_json() or {}))
        
        project = ProjectManager.get_project(req.project_id)
        if not project:
            return jsonify({"success": False, "error": "Project not found"}), 404
            
        graph_id = req.graph_id or project.graph_id
        if not graph_id:
            return jsonify({"success": False, "error": "Project graph not built"}), 400
        
        service = get_simulation_service()
        result = service.create_simulation(
            project_id=req.project_id,
            graph_id=graph_id,
            enable_twitter=req.enable_twitter,
            enable_reddit=req.enable_reddit
        )
        
        return result_to_response(result)
        
    except ValidationError as e:
        return jsonify({"success": False, "error": e.errors()}), 400
    except Exception as e:
        logger.error(f"Create simulation failed: {e}")
        return jsonify({"success": False, "error": str(e)}), 500


@routes_bp.route('/prepare', methods=['POST'])
@login_required
def prepare_simulation():
    try:
        req = SimulationPrepareRequest(**(request.get_json() or {}))
            
        manager = SimulationManager()
        state = manager.get_simulation(req.simulation_id)
        if not state:
            return jsonify({"success": False, "error": "Simulation not found"}), 404
            
        project = ProjectManager.get_project(state.project_id)
        if not project:
            return jsonify({"success": False, "error": "Project not found"}), 404
        
        service = get_simulation_service()
        result = service.start_preparation_task(
            simulation_id=req.simulation_id,
            project_id=project.project_id,
            graph_id=state.graph_id,
            simulation_requirement=project.simulation_requirement,
            document_text=ProjectManager.get_extracted_text(state.project_id) or "",
            entity_types=req.entity_types,
            selected_entity_ids=req.selected_entity_ids,
            use_llm_for_profiles=req.use_llm_for_profiles,
            parallel_profile_count=req.parallel_profile_count,
            force_regenerate=req.force_regenerate
        )
        
        return result_to_response(result)
        
    except ValidationError as e:
        return jsonify({"success": False, "error": e.errors()}), 400
    except Exception as e:
        logger.error(f"Prepare failed: {e}")
        return jsonify({"success": False, "error": str(e)}), 500


@routes_bp.route('/prepare/status', methods=['POST'])
@login_required
def get_prepare_status():
    try:
        req = SimulationStatusRequest(**(request.get_json() or {}))
        
        if req.simulation_id:
            service = get_simulation_service()
            result = service.check_prepared(req.simulation_id)
            
            if result.ok and result.value.is_prepared:
                info = result.value
                return jsonify({
                    "success": True, 
                    "data": {
                        "status": "ready",
                        "progress": 100,
                        "already_prepared": True,
                        "prepare_information": asdict(info)
                    }
                })
        
        if req.task_id:
            task = TaskManager().get_task(req.task_id)
            if task:
                return jsonify({"success": True, "data": task.to_dict()})
                
        return jsonify({"success": False, "error": "Task not found"}), 404
        
    except ValidationError as e:
        return jsonify({"success": False, "error": e.errors()}), 400
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500


@routes_bp.route('/list', methods=['GET'])
def list_simulations():
    try:
        project_id = request.args.get('project_id')
        manager = SimulationManager()
        results = manager.list_simulations(project_id)
        return jsonify({
            "success": True,
            "data": [s.to_dict() for s in results]
        })
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500
