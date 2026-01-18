"""
Simulation Related API Routes
Step2: Neo4j entities read with filter, OASIS simulation preparation and running (fully automated)
"""

import os
import traceback
from flask import request, jsonify, send_file

from . import simulation_bp
from ..config import Config
from ..services.neo4j_entity_reader import Neo4jEntityReader
from ..services.oasis_profile_generator import OasisProfileGenerator
from ..services.simulation_manager import SimulationManager, SimulationStatus
from ..services.simulation_runner import SimulationRunner, RunnerStatus
from ..services.panel_chat_service import PanelChatService
from ..services.survey_service import SurveyService
from ..services.agora_service import AgoraService, DebateStatus, DEBATE_TEMPLATES
from ..utils.logger import get_logger
from ..models.project import ProjectManager

logger = get_logger('pubop.api.simulation')


# Interview prompt optimization prefix
# Add this prefix to prevent Agent from calling tools, respond directly with text
INTERVIEW_PROMPT_PREFIX = "Based on your persona, past memories and actions, respond directly with text without calling any tools: "


def optimize_interview_prompt(prompt: str) -> str:
    """
    Optimize interview prompt by adding prefix to prevent Agent from calling tools
    
    Args:
        prompt: Original prompt
        
    Returns:
        Optimized prompt
    """
    if not prompt:
        return prompt
    # Avoid adding prefix repeatedly
    if prompt.startswith(INTERVIEW_PROMPT_PREFIX):
        return prompt
    return f"{INTERVIEW_PROMPT_PREFIX}{prompt}"


# ============== Entity Read Interface ==============

@simulation_bp.route('/entities/<graph_id>', methods=['GET'])
def get_graph_entities(graph_id: str):
    """
    Get all entities of the graph (filtered)
    
    Only returns nodes matching predefined entity types (Labels, not just Entity nodes)
    
    QueryArgs:
        entity_types: Comma-separated list of entity types (optional, for further filtering)
        enrich: Whether to get related edge information (default true)
    """
    try:
        if not Config.NEO4J_URI:
            return jsonify({
                "success": False,
                "error": "NEO4J_URI not configured"
            }), 500
        
        entity_types_str = request.args.get('entity_types', '')
        entity_types = [t.strip() for t in entity_types_str.split(',') if t.strip()] if entity_types_str else None
        enrich = request.args.get('enrich', 'true').lower() == 'true'
        
        logger.info(f"Getting graph entities: graph_id={graph_id}, entity_types={entity_types}, enrich={enrich}")
        
        reader = Neo4jEntityReader()
        result = reader.filter_defined_entities(
            graph_id=graph_id,
            defined_entity_types=entity_types,
            enrich_with_edges=enrich
        )
        
        return jsonify({
            "success": True,
            "data": result.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Get graph entities failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/entities/<graph_id>/<entity_uuid>', methods=['GET'])
def get_entity_detail(graph_id: str, entity_uuid: str):
    """Get single entity detailed information"""
    try:
        if not Config.NEO4J_URI:
            return jsonify({
                "success": False,
                "error": "NEO4J_URI not configured"
            }), 500
        
        reader = Neo4jEntityReader()
        entity = reader.get_entity_with_context(graph_id, entity_uuid)
        
        if not entity:
            return jsonify({
                "success": False,
                "error": f"Entity does not exist: {entity_uuid}"
            }), 404
        
        return jsonify({
            "success": True,
            "data": entity.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Get entity details failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/entities/<graph_id>/by-type/<entity_type>', methods=['GET'])
def get_entities_by_type(graph_id: str, entity_type: str):
    """Get all entities of the specified type"""
    try:
        if not Config.NEO4J_URI:
            return jsonify({
                "success": False,
                "error": "NEO4J_URI not configured"
            }), 500
        
        enrich = request.args.get('enrich', 'true').lower() == 'true'
        
        reader = Neo4jEntityReader()
        entities = reader.get_entities_by_type(
            graph_id=graph_id,
            entity_type=entity_type,
            enrich_with_edges=enrich
        )
        
        return jsonify({
            "success": True,
            "data": {
                "entity_type": entity_type,
                "count": len(entities),
                "entities": [e.to_dict() for e in entities]
            }
        })
        
    except Exception as e:
        logger.error(f"Get entity failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


# ============== Simulation Management Interface ==============

@simulation_bp.route('/create', methods=['POST'])
def create_simulation():
    """
    Create a new simulation
    
    Note: max_rounds and other parameters are generated by LLM, no need to set manually
    
    Request (JSON):
        {
            "project_id": "proj_xxxx",      // Required
            "graph_id": "pubop_xxxx",    // Optional, if not provided will get from project
            "enable_twitter": true,          // Optional, default true
            "enable_reddit": true            // Optional, default true
        }
    
    Returns:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "project_id": "proj_xxxx",
                "graph_id": "pubop_xxxx",
                "status": "created",
                "enable_twitter": true,
                "enable_reddit": true,
                "created_at": "2025-12-01T10:00:00"
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        project_id = data.get('project_id')
        if not project_id:
            return jsonify({
                "success": False,
                "error": "please provide project_id"
            }), 400
        
        project = ProjectManager.get_project(project_id)
        if not project:
            return jsonify({
                "success": False,
                "error": f"Project does not exist: {project_id}"
            }), 404
        
        graph_id = data.get('graph_id') or project.graph_id
        if not graph_id:
            return jsonify({
                "success": False,
                "error": "Project has not built graph, please call /api/graph/build first"
            }), 400
        
        manager = SimulationManager()
        state = manager.create_simulation(
            project_id=project_id,
            graph_id=graph_id,
            enable_twitter=data.get('enable_twitter', True),
            enable_reddit=data.get('enable_reddit', True),
        )
        
        return jsonify({
            "success": True,
            "data": state.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Create simulation failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


def _check_simulation_prepared(simulation_id: str) -> tuple:
    """
    Check if simulation is already prepared
    
    Checks:
    1. state.json exists and status is "ready"
    2. Required files exist: reddit_profiles.json, twitter_profiles.csv, simulation_config.json
    
    Note: Running scripts (run_*.py) are kept in backend/scripts/ directory, no longer copied to simulation directory
    
    Args:
        simulation_id: Simulation ID
        
    Returns:
        (is_prepared: bool, information: dict)
    """
    import os
    from ..config import Config
    
    simulation_dir = os.path.join(Config.OASIS_SIMULATION_DATA_DIR, simulation_id)
    
    # Check if directory exists
    if not os.path.exists(simulation_dir):
        return False, {"reason": "Simulation directory does not exist"}
    
    # Required files (not package, scripts located in backend/scripts/)
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
        return False, {
            "reason": "Missing required files",
            "missing_files": missing_files,
            "existing_files": existing_files
        }
    
    # Check state.json status
    state_file = os.path.join(simulation_dir, "state.json")
    try:
        import json
        with open(state_file, 'r', encoding='utf-8') as f:
            state_data = json.load(f)
        
        status = state_data.get("status", "")
        config_generated = state_data.get("config_generated", False)
        
        # detailedlog
        logger.debug(f"Checking simulation preparation status: {simulation_id}, status={status}, config_generated={config_generated}")
        
        # If config_generated=True and files exist, consider preparation completed
        # The following statuses indicate preparation work is completed:
        # - ready: Preparation completed, can run
        # - preparing: If config_generated=True, preparation is completed
        # - running: Currently running, preparation was completed earlier
        # - completed: Run completed, preparation was completed earlier
        # - stopped: Already stopped, preparation was completed earlier
        # - failed: Run failed (but preparation was completed)
        prepared_statuses = ["ready", "preparing", "running", "completed", "stopped", "failed"]
        if status in prepared_statuses and config_generated:
            # Get file statistics information
            profiles_file = os.path.join(simulation_dir, "reddit_profiles.json")
            config_file = os.path.join(simulation_dir, "simulation_config.json")
            
            profiles_count = 0
            if os.path.exists(profiles_file):
                with open(profiles_file, 'r', encoding='utf-8') as f:
                    profiles_data = json.load(f)
                    profiles_count = len(profiles_data) if isinstance(profiles_data, list) else 0
            
            # If status is preparing but files are completed, auto-update status to ready
            if status == "preparing":
                try:
                    state_data["status"] = "ready"
                    from datetime import datetime
                    state_data["updated_at"] = datetime.now().isoformat()
                    with open(state_file, 'w', encoding='utf-8') as f:
                        json.dump(state_data, f, ensure_ascii=False, indent=2)
                    logger.info(f"Auto-updated simulation status: {simulation_id} preparing -> ready")
                    status = "ready"
                except Exception as e:
                    logger.warning(f"Auto-update status failed: {e}")
            
            logger.info(f"simulation {simulation_id} check result: preparation completed (status={status}, config_generated={config_generated})")
            return True, {
                "status": status,
                "entities_count": state_data.get("entities_count", 0),
                "profiles_count": profiles_count,
                "entity_types": state_data.get("entity_types", []),
                "config_generated": config_generated,
                "created_at": state_data.get("created_at"),
                "updated_at": state_data.get("updated_at"),
                "existing_files": existing_files
            }
        else:
            logger.warning(f"simulation {simulation_id} check result: not prepared (status={status}, config_generated={config_generated})")
            return False, {
                "reason": f"status not in prepared list or config_generated is false: status={status}, config_generated={config_generated}",
                "status": status,
                "config_generated": config_generated
            }
            
    except Exception as e:
        return False, {"reason": f"Failed to read status file: {str(e)}"}


@simulation_bp.route('/prepare/preview', methods=['POST'])
def prepare_preview():
    """
    Preview entities before preparation - returns list of entities for user selection
    
    This endpoint allows users to see all entities that would become agents,
    grouped by entity type, so they can select which ones to include.
    
    Request (JSON):
        {
            "simulation_id": "sim_xxxx",     // Required
            "entity_types": ["Person"]       // Optional: filter to specific types
        }
    
    Response:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "total_count": 219,
                "entities": [
                    {
                        "uuid": "entity_uuid_1",
                        "name": "Entity Name",
                        "type": "Person",
                        "summary": "Brief description...",
                        "labels": ["GraphNode", "Person"],
                        "relationship_count": 5
                    },
                    ...
                ],
                "by_type": {
                    "Person": {"count": 85, "uuids": ["uuid1", "uuid2", ...]},
                    "Organization": {"count": 42, "uuids": [...]},
                    ...
                }
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "Please provide simulation_id"
            }), 400
        
        manager = SimulationManager()
        state = manager.get_simulation(simulation_id)
        
        if not state:
            return jsonify({
                "success": False,
                "error": f"Simulation does not exist: {simulation_id}"
            }), 404
        
        entity_types_filter = data.get('entity_types')
        
        # Read entities from Neo4j graph
        reader = Neo4jEntityReader()
        filtered = reader.filter_defined_entities(
            graph_id=state.graph_id,
            defined_entity_types=entity_types_filter,
            enrich_with_edges=True  # We need edge info for relationship counts
        )
        
        # Build entity list with details
        entities_list = []
        by_type = {}
        
        for entity in filtered.entities:
            entity_type = entity.get_entity_type() or "Unknown"
            
            entity_data = {
                "uuid": entity.uuid,
                "name": entity.name,
                "type": entity_type,
                "summary": entity.summary,
                "labels": entity.labels,
                "relationship_count": len(entity.related_edges)
            }
            entities_list.append(entity_data)
            
            # Group by type
            if entity_type not in by_type:
                by_type[entity_type] = {"count": 0, "uuids": []}
            by_type[entity_type]["count"] += 1
            by_type[entity_type]["uuids"].append(entity.uuid)
        
        return jsonify({
            "success": True,
            "data": {
                "simulation_id": simulation_id,
                "graph_id": state.graph_id,
                "total_count": filtered.filtered_count,
                "entities": entities_list,
                "by_type": by_type,
                "entity_types": list(filtered.entity_types)
            }
        })
        
    except Exception as e:
        logger.error(f"Prepare preview failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/prepare', methods=['POST'])
def prepare_simulation():
    """
    preparesimulationenvironment（异步任t务，LLM智cangenerate所haveparameters）
    
    thisis a耗时操作，interfacewill立即returntask_id，
    use GET /api/simulation/prepare/status Query进度
    
    特性：
    - 自动检测completedofprepare工作，避免重复generate
    - if already preparecompleted，直接return already haveresult
    - supportforce重新generate（force_regenerate=true）
    
    步骤：
    1. checkwhether to already havecompletedofprepare工作
    2. fromZepgraphread并filterentities
    3. for每entitiesgenerateOASIS Agent Profile（带retry机制）
    4. LLM智cangeneratesimulationconfigure（带retry机制）
    5. saveconfigurefilesand预设脚本
    
    request（JSON）：
        {
            "simulation_id": "sim_xxxx",                   // Required，simulationID
            "entity_types": ["Student", "PublicFigure"],  // Optional，指定entitiestype
            "use_llm_for_profiles": true,                 // Optional，whether touseLLMgeneratepeople设
            "parallel_profile_count": 5,                  // Optional，parallelgeneratepeople设quantity，default5
            "force_regenerate": false                     // Optional，force重新generate，defaultfalse
        }
    
    Returns:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "task_id": "task_xxxx",           // 新任务时return
                "status": "preparing|ready",
                "message": "prepare任务alreadystart| already havecompletedofprepare工作",
                "already_prepared": true|false    // whether to already preparecompleted
            }
        }
    """
    import threading
    import os
    from ..models.task import TaskManager, TaskStatus
    from ..config import Config
    
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "please provide simulation_id"
            }), 400
        
        manager = SimulationManager()
        state = manager.get_simulation(simulation_id)
        
        if not state:
            return jsonify({
                "success": False,
                "error": f"Simulation does not exist: {simulation_id}"
            }), 404
        
        # checkwhether toforce重新generation
        force_regenerate = data.get('force_regenerate', False)
        logger.info(f"startprocessing /prepare request: simulation_id={simulation_id}, force_regenerate={force_regenerate}")
        
        # checkwhether toalready经preparecompleted（避免重复generation）
        if not force_regenerate:
            logger.debug(f"Checking if simulation {simulation_id} is already prepared...")
            is_prepared, prepare_information = _check_simulation_prepared(simulation_id)
            logger.debug(f"checkresult: is_prepared={is_prepared}, prepare_information={prepare_information}")
            if is_prepared:
                logger.info(f"simulation {simulation_id} already prepared, skipping duplicate generation")
                return jsonify({
                    "success": True,
                    "data": {
                        "simulation_id": simulation_id,
                        "status": "ready",
                        "message": "Already have completed preparation, no need to regenerate",
                        "already_prepared": True,
                        "prepare_information": prepare_information
                    }
                })
            else:
                logger.info(f"simulation {simulation_id} not prepared, will start preparation task")
        
        # fromprojectget必wantinformation
        project = ProjectManager.get_project(state.project_id)
        if not project:
            return jsonify({
                "success": False,
                "error": f"Project does not exist: {state.project_id}"
            }), 404
        
        # getsimulationrequirement
        simulation_requirement = project.simulation_requirement or ""
        if not simulation_requirement:
            return jsonify({
                "success": False,
                "error": "project missing simulation requirement description (simulation_requirement)"
            }), 400
        
        # get文档文本
        document_text = ProjectManager.get_extracted_text(state.project_id) or ""
        
        entity_types_list = data.get('entity_types')
        selected_entity_ids = data.get('selected_entity_ids')  # NEW: optional list of entity UUIDs to include
        use_llm_for_profiles = data.get('use_llm_for_profiles', True)
        parallel_profile_count = data.get('parallel_profile_count', 5)
        
        # ========== 同步getentityquantity（in后台任务start前） ==========
        # this样前端incallprepare后立即thencanget到预期Agenttotal
        try:
            logger.info(f"Synchronously getting entity count: graph_id={state.graph_id}")
            reader = Neo4jEntityReader()
            # quickreadentity（not需wantedgeinformation，只statisticsquantity）
            filtered_preview = reader.filter_defined_entities(
                graph_id=state.graph_id,
                defined_entity_types=entity_types_list,
                enrich_with_edges=False  # notgetedgeinformation，加quick度
            )
            # saveentityquantity到status（供前端立即get）
            state.entities_count = filtered_preview.filtered_count
            state.entity_types = list(filtered_preview.entity_types)
            logger.info(f"Expected entity count: {filtered_preview.filtered_count}, types: {filtered_preview.entity_types}")
        except Exception as e:
            logger.warning(f"Synchronous entity count fetch failed (will retry in background task): {e}")
            # failednot影响后续流程，后台任务will重新get
        
        # create异步任务
        task_manager = TaskManager()
        task_id = task_manager.create_task(
            task_type="simulation_prepare",
            metadata={
                "simulation_id": simulation_id,
                "project_id": state.project_id
            }
        )
        
        # updatesimulation status（contains预firstgetofentityquantity）
        state.status = SimulationStatus.PREPARING
        manager._save_simulation_state(state)
        
        # 定义后台任务
        def run_prepare():
            try:
                task_manager.update_task(
                    task_id,
                    status=TaskStatus.PROCESSING,
                    progress=0,
                    message="Starting to prepare simulation environment..."
                )
                
                # preparesimulation（带进度回调）
                # 存储阶段进度details
                stage_details = {}
                
                def progress_callback(stage, progress, message, **kwargs):
                    # Calculate total progress
                    stage_weights = {
                        "reading": (0, 15),              # 0-15%
                        "scraping_realworld": (15, 35),  # 15-35% (NEW: real-world scraping)
                        "generating_profiles": (35, 70), # 35-70%
                        "generating_config": (70, 95),   # 70-95%
                        "copying_scripts": (95, 100)     # 95-100%
                    }
                    
                    start, end = stage_weights.get(stage, (0, 100))
                    current_progress = int(start + (end - start) * progress / 100)
                    
                    # Build detailed progress information
                    stage_names = {
                        "reading": "Reading graph entities",
                        "scraping_realworld": "Scraping real-world news",
                        "generating_profiles": "Generating Agent profiles",
                        "generating_config": "Generating simulation config",
                        "copying_scripts": "Preparing simulation scripts"
                    }
                    
                    stage_index = list(stage_weights.keys()).index(stage) + 1 if stage in stage_weights else 1
                    total_stages = len(stage_weights)
                    
                    # Update stage details
                    stage_details[stage] = {
                        "stage_name": stage_names.get(stage, stage),
                        "stage_progress": progress,
                        "current": kwargs.get("current", 0),
                        "total": kwargs.get("total", 0),
                        "item_name": kwargs.get("item_name", "")
                    }
                    
                    # Build detailed progress information
                    detail = stage_details[stage]
                    progress_detail_data = {
                        "current_stage": stage,
                        "current_stage_name": stage_names.get(stage, stage),
                        "stage_index": stage_index,
                        "total_stages": total_stages,
                        "stage_progress": progress,
                        "current_item": detail["current"],
                        "total_items": detail["total"],
                        "item_description": message
                    }
                    
                    # Build concise message
                    if detail["total"] > 0:
                        detailed_message = (
                            f"[{stage_index}/{total_stages}] {stage_names.get(stage, stage)}: "
                            f"{detail['current']}/{detail['total']} - {message}"
                        )
                    else:
                        detailed_message = f"[{stage_index}/{total_stages}] {stage_names.get(stage, stage)}: {message}"
                    
                    task_manager.update_task(
                        task_id,
                        progress=current_progress,
                        message=detailed_message,
                        progress_detail=progress_detail_data
                    )
                
                result_state = manager.prepare_simulation(
                    simulation_id=simulation_id,
                    simulation_requirement=simulation_requirement,
                    document_text=document_text,
                    defined_entity_types=entity_types_list,
                    selected_entity_ids=selected_entity_ids,  # NEW: pass selected entities
                    use_llm_for_profiles=use_llm_for_profiles,
                    progress_callback=progress_callback,
                    parallel_profile_count=parallel_profile_count
                )
                
                # Task completed
                task_manager.complete_task(
                    task_id,
                    result=result_state.to_simple_dict()
                )
                
            except Exception as e:
                logger.error(f"Prepare simulation failed: {str(e)}")
                task_manager.fail_task(task_id, str(e))
                
                # Update simulation status to failed
                state = manager.get_simulation(simulation_id)
                if state:
                    state.status = SimulationStatus.FAILED
                    state.error = str(e)
                    manager._save_simulation_state(state)
        
        # Start background thread
        thread = threading.Thread(target=run_prepare, daemon=True)
        thread.start()
        
        return jsonify({
            "success": True,
            "data": {
                "simulation_id": simulation_id,
                "task_id": task_id,
                "status": "preparing",
                "message": "Preparation task already started, please query progress via /api/simulation/prepare/status",
                "already_prepared": False,
                "expected_entities_count": state.entities_count,  # 预期ofAgenttotal
                "entity_types": state.entity_types  # entity typeslist
            }
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 404
        
    except Exception as e:
        logger.error(f"Start preparation task failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/prepare/status', methods=['POST'])
def get_prepare_status():
    """
    Queryprepare任务进度
    
    support两种Query方式：
    1. throughtask_idQuery正in进行of任务进度
    2. throughsimulation_idcheckwhether to already havecompletedofprepare工作
    
    request（JSON）：
        {
            "task_id": "task_xxxx",          // Optional，preparereturnoftask_id
            "simulation_id": "sim_xxxx"      // Optional，simulationID（use于checkcompletedofprepare）
        }
    
    Returns:
        {
            "success": true,
            "data": {
                "task_id": "task_xxxx",
                "status": "processing|completed|ready",
                "progress": 45,
                "message": "...",
                "already_prepared": true|false,  // whether to already havecompletedofprepare
                "prepare_information": {...}            //  already preparecompleted时ofdetailed informationrmation
            }
        }
    """
    from ..models.task import TaskManager
    
    try:
        data = request.get_json() or {}
        
        task_id = data.get('task_id')
        simulation_id = data.get('simulation_id')
        
        # if提供simulation_id，firstcheckwhether toalreadypreparecompleted
        if simulation_id:
            is_prepared, prepare_information = _check_simulation_prepared(simulation_id)
            if is_prepared:
                return jsonify({
                    "success": True,
                    "data": {
                        "simulation_id": simulation_id,
                        "status": "ready",
                        "progress": 100,
                        "message": "alreadyhavecompletedofprepare工作",
                        "already_prepared": True,
                        "prepare_information": prepare_information
                    }
                })
        
        # if没havetask_id，returnerror
        if not task_id:
            if simulation_id:
                # havesimulation_idbutnotpreparecompleted
                return jsonify({
                    "success": True,
                    "data": {
                        "simulation_id": simulation_id,
                        "status": "not_started",
                        "progress": 0,
                        "message": "尚notstartprepare，请call /api/simulation/prepare start",
                        "already_prepared": False
                    }
                })
            return jsonify({
                "success": False,
                "error": "please provide task_id  or  simulation_id"
            }), 400
        
        task_manager = TaskManager()
        task = task_manager.get_task(task_id)
        
        if not task:
            # 任务does not exist，butifhavesimulation_id，checkwhether toalreadypreparecompleted
            if simulation_id:
                is_prepared, prepare_information = _check_simulation_prepared(simulation_id)
                if is_prepared:
                    return jsonify({
                        "success": True,
                        "data": {
                            "simulation_id": simulation_id,
                            "task_id": task_id,
                            "status": "ready",
                            "progress": 100,
                            "message": "任务alreadycompleted（prepare工作already exists）",
                            "already_prepared": True,
                            "prepare_information": prepare_information
                        }
                    })
            
            return jsonify({
                "success": False,
                "error": f"任务does not exist: {task_id}"
            }), 404
        
        task_dict = task.to_dict()
        task_dict["already_prepared"] = False
        
        return jsonify({
            "success": True,
            "data": task_dict
        })
        
    except Exception as e:
        logger.error(f"Query task status failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@simulation_bp.route('/<simulation_id>', methods=['GET'])
def get_simulation(simulation_id: str):
    """getsimulation status"""
    try:
        manager = SimulationManager()
        state = manager.get_simulation(simulation_id)
        
        if not state:
            return jsonify({
                "success": False,
                "error": f"simulationdoes not exist: {simulation_id}"
            }), 404
        
        result = state.to_dict()
        
        # ifsimulationalreadyprepare好，附加runningsay明
        if state.status == SimulationStatus.READY:
            result["run_instructions"] = manager.get_run_instructions(simulation_id)
        
        return jsonify({
            "success": True,
            "data": result
        })
        
    except Exception as e:
        logger.error(f"getsimulation statusfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/list', methods=['GET'])
def list_simulations():
    """
    列出所havesimulation
    
    QueryArgs:
        project_id: Byproject IDfilter（Optional）
    """
    try:
        project_id = request.args.get('project_id')
        
        manager = SimulationManager()
        simulations = manager.list_simulations(project_id=project_id)
        
        return jsonify({
            "success": True,
            "data": [s.to_dict() for s in simulations],
            "count": len(simulations)
        })
        
    except Exception as e:
        logger.error(f"List simulations failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/profiles', methods=['GET'])
def get_simulation_profiles(simulation_id: str):
    """
    getsimulationofAgent Profile
    
    QueryArgs:
        platform: platform type（reddit/twitter，defaultreddit）
    """
    try:
        platform = request.args.get('platform', 'reddit')
        
        manager = SimulationManager()
        profiles = manager.get_profiles(simulation_id, platform=platform)
        
        return jsonify({
            "success": True,
            "data": {
                "platform": platform,
                "count": len(profiles),
                "profiles": profiles
            }
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 404
        
    except Exception as e:
        logger.error(f"getProfilefailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/profiles/realtime', methods=['GET'])
def get_simulation_profiles_realtime(simulation_id: str):
    """
    real-timegetsimulationofAgent Profile（use于ingenerate过程 real-time查look进度）
    
    with /profiles interfaceof区别：
    - 直接readfiles，not经过 SimulationManager
    - 适use于generate过程 ofreal-time查look
    - return额外of元count据（such asfilesmodifytime、whether to正ingenerateetc）
    
    QueryArgs:
        platform: platform type（reddit/twitter，defaultreddit）
    
    Returns:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "platform": "reddit",
                "count": 15,
                "total_expected": 93,  // 预期total（ifhave）
                "is_generating": true,  // whether to正ingenerate
                "file_exists": true,
                "file_modified_at": "2025-12-04T18:20:00",
                "profiles": [...]
            }
        }
    """
    import json
    import csv
    from datetime import datetime
    
    try:
        platform = request.args.get('platform', 'reddit')
        
        # getsimulationdirectory
        sim_dir = os.path.join(Config.OASIS_SIMULATION_DATA_DIR, simulation_id)
        
        if not os.path.exists(sim_dir):
            return jsonify({
                "success": False,
                "error": f"simulationdoes not exist: {simulation_id}"
            }), 404
        
        # 确定files路径
        if platform == "reddit":
            profiles_file = os.path.join(sim_dir, "reddit_profiles.json")
        else:
            profiles_file = os.path.join(sim_dir, "twitter_profiles.csv")
        
        # checkfileswhether to存in
        file_exists = os.path.exists(profiles_file)
        profiles = []
        file_modified_at = None
        
        if file_exists:
            # getfilesmodifytime
            file_stat = os.stat(profiles_file)
            file_modified_at = datetime.fromtimestamp(file_stat.st_mtime).isoformat()
            
            try:
                if platform == "reddit":
                    with open(profiles_file, 'r', encoding='utf-8') as f:
                        profiles = json.load(f)
                else:
                    with open(profiles_file, 'r', encoding='utf-8') as f:
                        reader = csv.DictReader(f)
                        profiles = list(reader)
            except (json.JSONDecodeError, Exception) as e:
                logger.warning(f"read profiles filesfailed（ can canin progresswrite ）: {e}")
                profiles = []
        
        # checkwhether toin progressgeneration（through state.json 判断）
        is_generating = False
        total_expected = None
        
        state_file = os.path.join(sim_dir, "state.json")
        if os.path.exists(state_file):
            try:
                with open(state_file, 'r', encoding='utf-8') as f:
                    state_data = json.load(f)
                    status = state_data.get("status", "")
                    is_generating = status == "preparing"
                    total_expected = state_data.get("entities_count")
            except Exception:
                pass
        
        return jsonify({
            "success": True,
            "data": {
                "simulation_id": simulation_id,
                "platform": platform,
                "count": len(profiles),
                "total_expected": total_expected,
                "is_generating": is_generating,
                "file_exists": file_exists,
                "file_modified_at": file_modified_at,
                "profiles": profiles
            }
        })
        
    except Exception as e:
        logger.error(f"Realtime get profile failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/config/realtime', methods=['GET'])
def get_simulation_config_realtime(simulation_id: str):
    """
    real-timegetsimulationconfigure（use于ingenerate过程 real-time查look进度）
    
    with /config interfaceof区别：
    - 直接readfiles，not经过 SimulationManager
    - 适use于generate过程 ofreal-time查look
    - return额外of元count据（such asfilesmodifytime、whether to正ingenerateetc）
    - 即使configure还没generate完alsocanreturn部分information
    
    Returns:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "file_exists": true,
                "file_modified_at": "2025-12-04T18:20:00",
                "is_generating": true,  // whether to正ingenerate
                "generation_stage": "generating_config",  // Currentgenerate阶段
                "config": {...}  // configurecontent（if存in）
            }
        }
    """
    import json
    from datetime import datetime
    
    try:
        # getsimulationdirectory
        sim_dir = os.path.join(Config.OASIS_SIMULATION_DATA_DIR, simulation_id)
        
        if not os.path.exists(sim_dir):
            return jsonify({
                "success": False,
                "error": f"simulationdoes not exist: {simulation_id}"
            }), 404
        
        # configurationfiles路径
        config_file = os.path.join(sim_dir, "simulation_config.json")
        
        # checkfileswhether to存in
        file_exists = os.path.exists(config_file)
        config = None
        file_modified_at = None
        
        if file_exists:
            # getfilesmodifytime
            file_stat = os.stat(config_file)
            file_modified_at = datetime.fromtimestamp(file_stat.st_mtime).isoformat()
            
            try:
                with open(config_file, 'r', encoding='utf-8') as f:
                    config = json.load(f)
            except (json.JSONDecodeError, Exception) as e:
                logger.warning(f"read config filesfailed（ can canin progresswrite ）: {e}")
                config = None
        
        # checkwhether toin progressgeneration（through state.json 判断）
        is_generating = False
        generation_stage = None
        config_generated = False
        
        state_file = os.path.join(sim_dir, "state.json")
        if os.path.exists(state_file):
            try:
                with open(state_file, 'r', encoding='utf-8') as f:
                    state_data = json.load(f)
                    status = state_data.get("status", "")
                    is_generating = status == "preparing"
                    config_generated = state_data.get("config_generated", False)
                    
                    # 判断Current阶段
                    if is_generating:
                        if state_data.get("profiles_generated", False):
                            generation_stage = "generating_config"
                        else:
                            generation_stage = "generating_profiles"
                    elif status == "ready":
                        generation_stage = "completed"
            except Exception:
                pass
        
        # 构建returncount据
        response_data = {
            "simulation_id": simulation_id,
            "file_exists": file_exists,
            "file_modified_at": file_modified_at,
            "is_generating": is_generating,
            "generation_stage": generation_stage,
            "config_generated": config_generated,
            "config": config
        }
        
        # ifconfiguration存in，Extract一些关keystatisticsinformation
        if config:
            response_data["summary"] = {
                "total_agents": len(config.get("agent_configs", [])),
                "simulation_hours": config.get("time_config", {}).get("total_simulation_hours"),
                "initial_posts_count": len(config.get("event_config", {}).get("initial_posts", [])),
                "hot_topics_count": len(config.get("event_config", {}).get("hot_topics", [])),
                "has_twitter_config": "twitter_config" in config,
                "has_reddit_config": "reddit_config" in config,
                "generated_at": config.get("generated_at"),
                "llm_model": config.get("llm_model")
            }
        
        return jsonify({
            "success": True,
            "data": response_data
        })
        
    except Exception as e:
        logger.error(f"Realtime get config failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/config', methods=['GET'])
def get_simulation_config(simulation_id: str):
    """
    getsimulationconfigure（LLM智cangenerateofcompleteconfigure）
    
    returncontains：
        - time_config: timeconfigure（simulation时长、轮times、高峰/低谷时段）
        - agent_configs: 每Agentof活动configure（活跃度、发言频率、立场etc）
        - event_config: 事件configure（初始posts、热点话题）
        - platform_configs: 平台configure
        - generation_reasoning: LLMofconfigure推理say明
    """
    try:
        manager = SimulationManager()
        config = manager.get_simulation_config(simulation_id)
        
        if not config:
            return jsonify({
                "success": False,
                "error": f"simulationconfigurationdoes not exist，please call first /prepare interface"
            }), 404
        
        return jsonify({
            "success": True,
            "data": config
        })
        
    except Exception as e:
        logger.error(f"getconfigurationfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/config/download', methods=['GET'])
def download_simulation_config(simulation_id: str):
    """downloadsimulationconfigurationfiles"""
    try:
        manager = SimulationManager()
        sim_dir = manager._get_simulation_dir(simulation_id)
        config_path = os.path.join(sim_dir, "simulation_config.json")
        
        if not os.path.exists(config_path):
            return jsonify({
                "success": False,
                "error": "configurationfilesdoes not exist，please call first /prepare interface"
            }), 404
        
        return send_file(
            config_path,
            as_attachment=True,
            download_name="simulation_config.json"
        )
        
    except Exception as e:
        logger.error(f"downloadconfigurationfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/script/<script_name>/download', methods=['GET'])
def download_simulation_script(script_name: str):
    """
    downloadsimulationrunning脚本files（通use脚本，位于 backend/scripts/）
    
    script_nameOptionalvalue：
        - run_twitter_simulation.py
        - run_reddit_simulation.py
        - run_parallel_simulation.py
        - action_logger.py
    """
    try:
        # 脚本位于 backend/scripts/ directory
        scripts_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), '../../scripts'))
        
        # validate脚本名称
        allowed_scripts = [
            "run_twitter_simulation.py",
            "run_reddit_simulation.py", 
            "run_parallel_simulation.py",
            "action_logger.py"
        ]
        
        if script_name not in allowed_scripts:
            return jsonify({
                "success": False,
                "error": f"not知脚本: {script_name}，Optional: {allowed_scripts}"
            }), 400
        
        script_path = os.path.join(scripts_dir, script_name)
        
        if not os.path.exists(script_path):
            return jsonify({
                "success": False,
                "error": f"脚本filesdoes not exist: {script_name}"
            }), 404
        
        return send_file(
            script_path,
            as_attachment=True,
            download_name=script_name
        )
        
    except Exception as e:
        logger.error(f"Download script failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


# ============== Profile Generation Interface (Standalone) ==============

@simulation_bp.route('/generate-profiles', methods=['POST'])
def generate_profiles():
    """
    直接fromgraphgenerateOASIS Agent Profile（notcreatesimulation）
    
    request（JSON）：
        {
            "graph_id": "pubop_xxxx",     // Required
            "entity_types": ["Student"],      // Optional
            "use_llm": true,                  // Optional
            "platform": "reddit"              // Optional
        }
    """
    try:
        data = request.get_json() or {}
        
        graph_id = data.get('graph_id')
        if not graph_id:
            return jsonify({
                "success": False,
                "error": "please provide graph_id"
            }), 400
        
        entity_types = data.get('entity_types')
        use_llm = data.get('use_llm', True)
        platform = data.get('platform', 'reddit')
        
        reader = Neo4jEntityReader()
        filtered = reader.filter_defined_entities(
            graph_id=graph_id,
            defined_entity_types=entity_types,
            enrich_with_edges=True
        )
        
        if filtered.filtered_count == 0:
            return jsonify({
                "success": False,
                "error": "没have找到符合件ofentity"
            }), 400
        
        generator = OasisProfileGenerator()
        profiles = generator.generate_profiles_from_entities(
            entities=filtered.entities,
            use_llm=use_llm
        )
        
        if platform == "reddit":
            profiles_data = [p.to_reddit_format() for p in profiles]
        elif platform == "twitter":
            profiles_data = [p.to_twitter_format() for p in profiles]
        else:
            profiles_data = [p.to_dict() for p in profiles]
        
        return jsonify({
            "success": True,
            "data": {
                "platform": platform,
                "entity_types": list(filtered.entity_types),
                "count": len(profiles_data),
                "profiles": profiles_data
            }
        })
        
    except Exception as e:
        logger.error(f"generationProfilefailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


# ============== Simulation Running Control Interface ==============

@simulation_bp.route('/start', methods=['POST'])
def start_simulation():
    """
    startrunningsimulation

    request（JSON）：
        {
            "simulation_id": "sim_xxxx",          // Required，simulationID
            "platform": "parallel",                // Optional: twitter / reddit / parallel (default)
            "max_rounds": 100,                     // Optional: maximumsimulation轮count，use于截断过长ofsimulation
            "enable_graph_memory_update": false,   // Optional: whether to will Agent活动动态update到Zepgraph记忆
            "force": false                         // Optional: force重新start（willstoppedrunningofsimulation并清理log）
        }

    about force Args:
        - 启use后，ifsimulation正inrunning or completed，willfirststopped并清理running log
        - 清理ofcontentpackage括：run_state.json, actions.jsonl, simulation.log etc
        - notwill清理configurefiles（simulation_config.json）and profile files
        - 适use于需want重新runningsimulationof场景

    about enable_graph_memory_update：
        - 启use后，simulation 所haveAgentof活动（发帖、comments、点赞etc）都willreal-timeupdate到Zepgraph
        - thiscan让graph"记住"simulation过程，use于后续分析 or AIdialogue
        - 需wantsimulation关联ofprojecthavehave效of graph_id
        - usebatchupdate机制，减少APIcalltimescount

    Returns:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "runner_status": "running",
                "process_pid": 12345,
                "twitter_running": true,
                "reddit_running": true,
                "started_at": "2025-12-01T10:00:00",
                "graph_memory_update_enabled": true,  // whether to启usegraph记忆update
                "force_restarted": true               // whether toisforce重新start
            }
        }
    """
    try:
        data = request.get_json() or {}

        simulation_id = data.get('simulation_id')
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "please provide simulation_id"
            }), 400

        platform = data.get('platform', 'parallel')
        max_rounds = data.get('max_rounds')  # Optional：maximumsimulation轮count
        enable_graph_memory_update = data.get('enable_graph_memory_update', False)  # Optional：whether to启usegraph记忆update
        force = data.get('force', False)  # Optional：force重新start

        # validate max_rounds parameters
        if max_rounds is not None:
            try:
                max_rounds = int(max_rounds)
                if max_rounds <= 0:
                    return jsonify({
                        "success": False,
                        "error": "max_rounds mustis正整count"
                    }), 400
            except (ValueError, TypeError):
                return jsonify({
                    "success": False,
                    "error": "max_rounds mustishave效of整count"
                }), 400

        if platform not in ['twitter', 'reddit', 'parallel']:
            return jsonify({
                "success": False,
                "error": f"invalidplatformtype: {platform}，Optional: twitter/reddit/parallel"
            }), 400

        # checksimulationwhether toalreadyprepare好
        manager = SimulationManager()
        state = manager.get_simulation(simulation_id)

        if not state:
            return jsonify({
                "success": False,
                "error": f"simulationdoes not exist: {simulation_id}"
            }), 404

        force_restarted = False
        
        # 智canprocessingstatus：ifprepare工作alreadycompleted，允许重新start
        if state.status != SimulationStatus.READY:
            # checkprepare工作whether toalreadycompleted
            is_prepared, prepare_information = _check_simulation_prepared(simulation_id)

            if is_prepared:
                # prepare工作alreadycompleted，checkwhether tohavein progressrunningofprocess
                if state.status == SimulationStatus.RUNNING:
                    # checksimulationprocesswhether to真ofinrunning
                    run_state = SimulationRunner.get_run_state(simulation_id)
                    if run_state and run_state.runner_status.value == "running":
                        # process确实inrunning
                        if force:
                            # forcemode：stoprunningofsimulation
                            logger.info(f"forcemode：stoprunningofsimulation {simulation_id}")
                            try:
                                SimulationRunner.stop_simulation(simulation_id)
                            except Exception as e:
                                logger.warning(f"Warning while stopping simulation: {str(e)}")
                        else:
                            return jsonify({
                                "success": False,
                                "error": f"simulationin progressrunning，please call first /stop interfacestop， or use force=true force重新start"
                            }), 400

                # ifisforcemode，清理running log
                if force:
                    logger.info(f"Forced mode: cleaning up simulation logs {simulation_id}")
                    cleanup_result = SimulationRunner.cleanup_simulation_logs(simulation_id)
                    if not cleanup_result.get("success"):
                        logger.warning(f"Warning while cleaning up logs: {cleanup_result.get('errors')}")
                    force_restarted = True

                # processdoes not exist or alreadyend，重置statusfor ready
                logger.info(f"simulation {simulation_id} preparation completed, resetting status to ready (original status: {state.status.value})")
                state.status = SimulationStatus.READY
                manager._save_simulation_state(state)
            else:
                # prepare工作notcompleted
                return jsonify({
                    "success": False,
                    "error": f"simulationnotprepare好，Currentstatus: {state.status.value}，please call first /prepare interface"
                }), 400
        
        # getgraphID（use于graph记忆update）
        graph_id = None
        if enable_graph_memory_update:
            # fromsimulation status or project get graph_id
            graph_id = state.graph_id
            if not graph_id:
                # 尝试fromproject get
                project = ProjectManager.get_project(state.project_id)
                if project:
                    graph_id = project.graph_id
            
            if not graph_id:
                return jsonify({
                    "success": False,
                    "error": "Enabling graph memory update requires a valid graph_id, please ensure project has built graph"
                }), 400
            
            logger.info(f"Enabling graph memory update: simulation_id={simulation_id}, graph_id={graph_id}")
        
        # startsimulation
        run_state = SimulationRunner.start_simulation(
            simulation_id=simulation_id,
            platform=platform,
            max_rounds=max_rounds,
            enable_graph_memory_update=enable_graph_memory_update,
            graph_id=graph_id
        )
        
        # updatesimulation status
        state.status = SimulationStatus.RUNNING
        manager._save_simulation_state(state)
        
        response_data = run_state.to_dict()
        if max_rounds:
            response_data['max_rounds_applied'] = max_rounds
        response_data['graph_memory_update_enabled'] = enable_graph_memory_update
        response_data['force_restarted'] = force_restarted
        if enable_graph_memory_update:
            response_data['graph_id'] = graph_id
        
        return jsonify({
            "success": True,
            "data": response_data
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
        
    except Exception as e:
        logger.error(f"startsimulationfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/stop', methods=['POST'])
def stop_simulation():
    """
    stoppedsimulation
    
    request（JSON）：
        {
            "simulation_id": "sim_xxxx"  // Required，simulationID
        }
    
    Returns:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "runner_status": "stopped",
                "completed_at": "2025-12-01T12:00:00"
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "please provide simulation_id"
            }), 400
        
        run_state = SimulationRunner.stop_simulation(simulation_id)
        
        # updatesimulation status
        manager = SimulationManager()
        state = manager.get_simulation(simulation_id)
        if state:
            state.status = SimulationStatus.PAUSED
            manager._save_simulation_state(state)
        
        return jsonify({
            "success": True,
            "data": run_state.to_dict()
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
        
    except Exception as e:
        logger.error(f"stopsimulationfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


# ============== Real-time Status Monitoring Interface ==============

@simulation_bp.route('/<simulation_id>/run-status', methods=['GET'])
def get_run_status(simulation_id: str):
    """
    getsimulationrunningreal-timestatus（use于前端轮询）
    
    Returns:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "runner_status": "running",
                "current_round": 5,
                "total_rounds": 144,
                "progress_percent": 3.5,
                "simulated_hours": 2,
                "total_simulation_hours": 72,
                "twitter_running": true,
                "reddit_running": true,
                "twitter_actions_count": 150,
                "reddit_actions_count": 200,
                "total_actions_count": 350,
                "started_at": "2025-12-01T10:00:00",
                "updated_at": "2025-12-01T10:30:00"
            }
        }
    """
    try:
        run_state = SimulationRunner.get_run_state(simulation_id)
        
        if not run_state:
            return jsonify({
                "success": True,
                "data": {
                    "simulation_id": simulation_id,
                    "runner_status": "idle",
                    "current_round": 0,
                    "total_rounds": 0,
                    "progress_percent": 0,
                    "twitter_actions_count": 0,
                    "reddit_actions_count": 0,
                    "total_actions_count": 0,
                }
            })
        
        return jsonify({
            "success": True,
            "data": run_state.to_dict()
        })
        
    except Exception as e:
        logger.error(f"getrunningstatusfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/run-status/detail', methods=['GET'])
def get_run_status_detail(simulation_id: str):
    """
    getsimulationrunningdetailedstatus（contains所haveaction）
    
    use于前端展示real-time动态
    
    QueryArgs:
        platform: filter平台（twitter/reddit，Optional）
    
    Returns:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "runner_status": "running",
                "current_round": 5,
                ...
                "all_actions": [
                    {
                        "round_num": 5,
                        "timestamp": "2025-12-01T10:30:00",
                        "platform": "twitter",
                        "agent_id": 3,
                        "agent_name": "Agent Name",
                        "action_type": "CREATE_POST",
                        "action_args": {"content": "..."},
                        "result": null,
                        "success": true
                    },
                    ...
                ],
                "twitter_actions": [...],  # Twitter platformof所haveaction
                "reddit_actions": [...]    # Reddit platformof所haveaction
            }
        }
    """
    try:
        run_state = SimulationRunner.get_run_state(simulation_id)
        platform_filter = request.args.get('platform')
        
        if not run_state:
            return jsonify({
                "success": True,
                "data": {
                    "simulation_id": simulation_id,
                    "runner_status": "idle",
                    "all_actions": [],
                    "twitter_actions": [],
                    "reddit_actions": []
                }
            })
        
        # getcompleteofactionlist
        all_actions = SimulationRunner.get_all_actions(
            simulation_id=simulation_id,
            platform=platform_filter
        )
        
        # 分platformgetaction
        twitter_actions = SimulationRunner.get_all_actions(
            simulation_id=simulation_id,
            platform="twitter"
        ) if not platform_filter or platform_filter == "twitter" else []
        
        reddit_actions = SimulationRunner.get_all_actions(
            simulation_id=simulation_id,
            platform="reddit"
        ) if not platform_filter or platform_filter == "reddit" else []
        
        # getCurrent轮timesofaction（recent_actions 只展示最新一轮）
        current_round = run_state.current_round
        recent_actions = SimulationRunner.get_all_actions(
            simulation_id=simulation_id,
            platform=platform_filter,
            round_num=current_round
        ) if current_round > 0 else []
        
        # get基础statusinformation
        result = run_state.to_dict()
        result["all_actions"] = [a.to_dict() for a in all_actions]
        result["twitter_actions"] = [a.to_dict() for a in twitter_actions]
        result["reddit_actions"] = [a.to_dict() for a in reddit_actions]
        result["rounds_count"] = len(run_state.rounds)
        # recent_actions 只展示Current最新一轮两platformofcontent
        result["recent_actions"] = [a.to_dict() for a in recent_actions]
        
        return jsonify({
            "success": True,
            "data": result
        })
        
    except Exception as e:
        logger.error(f"getdetailedstatusfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/actions', methods=['GET'])
def get_simulation_actions(simulation_id: str):
    """
    getsimulation ofAgentactionhistory
    
    QueryArgs:
        limit: return count（default100）
        offset: offset（default0）
        platform: filter平台（twitter/reddit）
        agent_id: filterAgent ID
        round_num: filter轮times
    
    Returns:
        {
            "success": true,
            "data": {
                "count": 100,
                "actions": [...]
            }
        }
    """
    try:
        limit = request.args.get('limit', 100, type=int)
        offset = request.args.get('offset', 0, type=int)
        platform = request.args.get('platform')
        agent_id = request.args.get('agent_id', type=int)
        round_num = request.args.get('round_num', type=int)
        
        actions = SimulationRunner.get_actions(
            simulation_id=simulation_id,
            limit=limit,
            offset=offset,
            platform=platform,
            agent_id=agent_id,
            round_num=round_num
        )
        
        return jsonify({
            "success": True,
            "data": {
                "count": len(actions),
                "actions": [a.to_dict() for a in actions]
            }
        })
        
    except Exception as e:
        logger.error(f"getactionhistoryfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/timeline', methods=['GET'])
def get_simulation_timeline(simulation_id: str):
    """
    getsimulationtime线（By轮times汇Total）
    
    use于前端展示进度andtime线view
    
    QueryArgs:
        start_round: 起始轮times（default0）
        end_round: end轮times（default全部）
    
    return每轮of汇Totalinformation
    """
    try:
        start_round = request.args.get('start_round', 0, type=int)
        end_round = request.args.get('end_round', type=int)
        
        timeline = SimulationRunner.get_timeline(
            simulation_id=simulation_id,
            start_round=start_round,
            end_round=end_round
        )
        
        return jsonify({
            "success": True,
            "data": {
                "rounds_count": len(timeline),
                "timeline": timeline
            }
        })
        
    except Exception as e:
        logger.error(f"Get timeline failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/agent-stats', methods=['GET'])
def get_agent_stats(simulation_id: str):
    """
    get每Agentofstatisticsinformation
    
    use于前端展示Agent活跃度排行、action分布etc
    """
    try:
        stats = SimulationRunner.get_agent_stats(simulation_id)
        
        return jsonify({
            "success": True,
            "data": {
                "agents_count": len(stats),
                "stats": stats
            }
        })
        
    except Exception as e:
        logger.error(f"getAgentstatisticsfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


# ============== Database Query Interface ==============

@simulation_bp.route('/<simulation_id>/posts', methods=['GET'])
def get_simulation_posts(simulation_id: str):
    """
    getsimulation ofposts
    
    QueryArgs:
        platform: platform type（twitter/reddit）
        limit: return count（default50）
        offset: offset
    
    returnpostslist（fromSQLitecount据libraryread）
    """
    try:
        platform = request.args.get('platform', 'reddit')
        limit = request.args.get('limit', 50, type=int)
        offset = request.args.get('offset', 0, type=int)
        
        sim_dir = os.path.join(
            os.path.dirname(__file__),
            f'../../uploads/simulations/{simulation_id}'
        )
        
        db_file = f"{platform}_simulation.db"
        db_path = os.path.join(sim_dir, db_file)
        
        if not os.path.exists(db_path):
            return jsonify({
                "success": True,
                "data": {
                    "platform": platform,
                    "count": 0,
                    "posts": [],
                    "message": "count据librarydoes not exist，simulation can can尚notrunning"
                }
            })
        
        import sqlite3
        conn = sqlite3.connect(db_path)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()
        
        try:
            cursor.execute("""
                SELECT * FROM post 
                ORDER BY created_at DESC 
                LIMIT ? OFFSET ?
            """, (limit, offset))
            
            posts = [dict(row) for row in cursor.fetchall()]
            
            cursor.execute("SELECT COUNT(*) FROM post")
            total = cursor.fetchone()[0]
            
        except sqlite3.OperationalError:
            posts = []
            total = 0
        
        conn.close()
        
        return jsonify({
            "success": True,
            "data": {
                "platform": platform,
                "total": total,
                "count": len(posts),
                "posts": posts
            }
        })
        
    except Exception as e:
        logger.error(f"Get posts failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/<simulation_id>/comments', methods=['GET'])
def get_simulation_comments(simulation_id: str):
    """
    getsimulation ofcomments（Reddit only）
    
    QueryArgs:
        post_id: filterpostsID（Optional）
        limit: return count
        offset: offset
    """
    try:
        post_id = request.args.get('post_id')
        limit = request.args.get('limit', 50, type=int)
        offset = request.args.get('offset', 0, type=int)
        
        sim_dir = os.path.join(
            os.path.dirname(__file__),
            f'../../uploads/simulations/{simulation_id}'
        )
        
        db_path = os.path.join(sim_dir, "reddit_simulation.db")
        
        if not os.path.exists(db_path):
            return jsonify({
                "success": True,
                "data": {
                    "count": 0,
                    "comments": []
                }
            })
        
        import sqlite3
        conn = sqlite3.connect(db_path)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()
        
        try:
            if post_id:
                cursor.execute("""
                    SELECT * FROM comment 
                    WHERE post_id = ?
                    ORDER BY created_at DESC 
                    LIMIT ? OFFSET ?
                """, (post_id, limit, offset))
            else:
                cursor.execute("""
                    SELECT * FROM comment 
                    ORDER BY created_at DESC 
                    LIMIT ? OFFSET ?
                """, (limit, offset))
            
            comments = [dict(row) for row in cursor.fetchall()]
            
        except sqlite3.OperationalError:
            comments = []
        
        conn.close()
        
        return jsonify({
            "success": True,
            "data": {
                "count": len(comments),
                "comments": comments
            }
        })
        
    except Exception as e:
        logger.error(f"Get comments failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


# ============== Interview Interface ==============

@simulation_bp.route('/interview', methods=['POST'])
def interview_agent():
    """
    interview单Agent

    Note:此function需wantsimulationenvironment处于runningstatus（completedsimulation循环后Enteretc待commandmode）

    request（JSON）：
        {
            "simulation_id": "sim_xxxx",       // Required，simulationID
            "agent_id": 0,                     // Required，Agent ID
            "prompt": "youtothis件事have什么look法？",  // Required，interview问题
            "platform": "twitter",             // Optional，指定平台（twitter/reddit）
                                               // not指定时：双平台simulation同时interview两平台
            "timeout": 60                      // Optional，timeouttime（秒），default60
        }

    return（not指定platform，双平台mode）：
        {
            "success": true,
            "data": {
                "agent_id": 0,
                "prompt": "youtothis件事have什么look法？",
                "result": {
                    "agent_id": 0,
                    "prompt": "...",
                    "platforms": {
                        "twitter": {"agent_id": 0, "response": "...", "platform": "twitter"},
                        "reddit": {"agent_id": 0, "response": "...", "platform": "reddit"}
                    }
                },
                "timestamp": "2025-12-08T10:00:01"
            }
        }

    return（指定platform）：
        {
            "success": true,
            "data": {
                "agent_id": 0,
                "prompt": "youtothis件事have什么look法？",
                "result": {
                    "agent_id": 0,
                    "response": "I认for...",
                    "platform": "twitter",
                    "timestamp": "2025-12-08T10:00:00"
                },
                "timestamp": "2025-12-08T10:00:01"
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        agent_id = data.get('agent_id')
        prompt = data.get('prompt')
        platform = data.get('platform')  # Optional：twitter/reddit/None
        timeout = data.get('timeout', 60)
        
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "please provide simulation_id"
            }), 400
        
        if agent_id is None:
            return jsonify({
                "success": False,
                "error": "please provide agent_id"
            }), 400
        
        if not prompt:
            return jsonify({
                "success": False,
                "error": "please provide prompt（interview问题）"
            }), 400
        
        # validateplatformparameters
        if platform and platform not in ("twitter", "reddit"):
            return jsonify({
                "success": False,
                "error": "platform parameters只canis 'twitter'  or  'reddit'"
            }), 400
        
        # checkenvironmentstatus
        if not SimulationRunner.check_env_alive(simulation_id):
            return jsonify({
                "success": False,
                "error": "simulationenvironmentnotrunning or alreadyclose。please ensuresimulation already completed并Enterwaitingcommandmode。"
            }), 400
        
        # 优化prompt，添加前缀避免Agentcalltool
        optimized_prompt = optimize_interview_prompt(prompt)
        
        result = SimulationRunner.interview_agent(
            simulation_id=simulation_id,
            agent_id=agent_id,
            prompt=optimized_prompt,
            platform=platform,
            timeout=timeout
        )

        return jsonify({
            "success": result.get("success", False),
            "data": result
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
        
    except TimeoutError as e:
        return jsonify({
            "success": False,
            "error": f"waitingInterviewresponsetimeout: {str(e)}"
        }), 504
        
    except Exception as e:
        logger.error(f"Interviewfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/interview/batch', methods=['POST'])
def interview_agents_batch():
    """
    batchinterview多Agent

    Note:此function需wantsimulationenvironment处于runningstatus

    request（JSON）：
        {
            "simulation_id": "sim_xxxx",       // Required，simulationID
            "interviews": [                    // Required，interviewlist
                {
                    "agent_id": 0,
                    "prompt": "youtoAhave什么look法？",
                    "platform": "twitter"      // Optional，指定该Agentofinterview平台
                },
                {
                    "agent_id": 1,
                    "prompt": "youtoBhave什么look法？"  // not指定platform则usedefaultvalue
                }
            ],
            "platform": "reddit",              // Optional，default平台（被每项ofplatform覆盖）
                                               // not指定时：双平台simulation每Agent同时interview两平台
            "timeout": 120                     // Optional，timeouttime（秒），default120
        }

    Returns:
        {
            "success": true,
            "data": {
                "interviews_count": 2,
                "result": {
                    "interviews_count": 4,
                    "results": {
                        "twitter_0": {"agent_id": 0, "response": "...", "platform": "twitter"},
                        "reddit_0": {"agent_id": 0, "response": "...", "platform": "reddit"},
                        "twitter_1": {"agent_id": 1, "response": "...", "platform": "twitter"},
                        "reddit_1": {"agent_id": 1, "response": "...", "platform": "reddit"}
                    }
                },
                "timestamp": "2025-12-08T10:00:01"
            }
        }
    """
    try:
        data = request.get_json() or {}

        simulation_id = data.get('simulation_id')
        interviews = data.get('interviews')
        platform = data.get('platform')  # Optional：twitter/reddit/None
        timeout = data.get('timeout', 120)

        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "please provide simulation_id"
            }), 400

        if not interviews or not isinstance(interviews, list):
            return jsonify({
                "success": False,
                "error": "please provide interviews（interviewlist）"
            }), 400

        # validateplatformparameters
        if platform and platform not in ("twitter", "reddit"):
            return jsonify({
                "success": False,
                "error": "platform parameters只canis 'twitter'  or  'reddit'"
            }), 400

        # validate每interview项
        for i, interview in enumerate(interviews):
            if 'agent_id' not in interview:
                return jsonify({
                    "success": False,
                    "error": f"interviewlist第{i+1}项缺少 agent_id"
                }), 400
            if 'prompt' not in interview:
                return jsonify({
                    "success": False,
                    "error": f"interviewlist第{i+1}项缺少 prompt"
                }), 400
            # validate每项ofplatform（ifhave）
            item_platform = interview.get('platform')
            if item_platform and item_platform not in ("twitter", "reddit"):
                return jsonify({
                    "success": False,
                    "error": f"interviewlist第{i+1}项ofplatform只canis 'twitter'  or  'reddit'"
                }), 400

        # checkenvironmentstatus
        if not SimulationRunner.check_env_alive(simulation_id):
            return jsonify({
                "success": False,
                "error": "simulationenvironmentnotrunning or alreadyclose。please ensuresimulation already completed并Enterwaitingcommandmode。"
            }), 400

        # 优化每interview项ofprompt，添加前缀避免Agentcalltool
        optimized_interviews = []
        for interview in interviews:
            optimized_interview = interview.copy()
            optimized_interview['prompt'] = optimize_interview_prompt(interview.get('prompt', ''))
            optimized_interviews.append(optimized_interview)

        result = SimulationRunner.interview_agents_batch(
            simulation_id=simulation_id,
            interviews=optimized_interviews,
            platform=platform,
            timeout=timeout
        )

        return jsonify({
            "success": result.get("success", False),
            "data": result
        })

    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400

    except TimeoutError as e:
        return jsonify({
            "success": False,
            "error": f"waitingbatchInterviewresponsetimeout: {str(e)}"
        }), 504

    except Exception as e:
        logger.error(f"Batch interview failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/interview/all', methods=['POST'])
def interview_all_agents():
    """
    全局interview - use相同问题interview所haveAgent

    Note:此function需wantsimulationenvironment处于runningstatus

    request（JSON）：
        {
            "simulation_id": "sim_xxxx",            // Required，simulationID
            "prompt": "youtothis件事整体have什么look法？",  // Required，interview问题（所haveAgentuse相同问题）
            "platform": "reddit",                   // Optional，指定平台（twitter/reddit）
                                                    // not指定时：双平台simulation每Agent同时interview两平台
            "timeout": 180                          // Optional，timeouttime（秒），default180
        }

    Returns:
        {
            "success": true,
            "data": {
                "interviews_count": 50,
                "result": {
                    "interviews_count": 100,
                    "results": {
                        "twitter_0": {"agent_id": 0, "response": "...", "platform": "twitter"},
                        "reddit_0": {"agent_id": 0, "response": "...", "platform": "reddit"},
                        ...
                    }
                },
                "timestamp": "2025-12-08T10:00:01"
            }
        }
    """
    try:
        data = request.get_json() or {}

        simulation_id = data.get('simulation_id')
        prompt = data.get('prompt')
        platform = data.get('platform')  # Optional：twitter/reddit/None
        timeout = data.get('timeout', 180)

        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "please provide simulation_id"
            }), 400

        if not prompt:
            return jsonify({
                "success": False,
                "error": "please provide prompt（interview问题）"
            }), 400

        # validateplatformparameters
        if platform and platform not in ("twitter", "reddit"):
            return jsonify({
                "success": False,
                "error": "platform parameters只canis 'twitter'  or  'reddit'"
            }), 400

        # checkenvironmentstatus
        if not SimulationRunner.check_env_alive(simulation_id):
            return jsonify({
                "success": False,
                "error": "simulationenvironmentnotrunning or alreadyclose。please ensuresimulation already completed并Enterwaitingcommandmode。"
            }), 400

        # 优化prompt，添加前缀避免Agentcalltool
        optimized_prompt = optimize_interview_prompt(prompt)

        result = SimulationRunner.interview_all_agents(
            simulation_id=simulation_id,
            prompt=optimized_prompt,
            platform=platform,
            timeout=timeout
        )

        return jsonify({
            "success": result.get("success", False),
            "data": result
        })

    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400

    except TimeoutError as e:
        return jsonify({
            "success": False,
            "error": f"waiting全局Interviewresponsetimeout: {str(e)}"
        }), 504

    except Exception as e:
        logger.error(f"Global interview failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/interview/history', methods=['POST'])
def get_interview_history():
    """
    getInterviewhistoryrecord

    fromsimulationcount据library read所haveInterviewrecord

    request（JSON）：
        {
            "simulation_id": "sim_xxxx",  // Required，simulationID
            "platform": "reddit",          // Optional，platform type（reddit/twitter）
                                           // not指定则return两平台of所havehistory
            "agent_id": 0,                 // Optional，只get该Agentofinterviewhistory
            "limit": 100                   // Optional，return count，default100
        }

    Returns:
        {
            "success": true,
            "data": {
                "count": 10,
                "history": [
                    {
                        "agent_id": 0,
                        "response": "I认for...",
                        "prompt": "youtothis件事have什么look法？",
                        "timestamp": "2025-12-08T10:00:00",
                        "platform": "reddit"
                    },
                    ...
                ]
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        platform = data.get('platform')  # not指定则return两platformofhistory
        agent_id = data.get('agent_id')
        limit = data.get('limit', 100)
        
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "please provide simulation_id"
            }), 400

        history = SimulationRunner.get_interview_history(
            simulation_id=simulation_id,
            platform=platform,
            agent_id=agent_id,
            limit=limit
        )

        return jsonify({
            "success": True,
            "data": {
                "count": len(history),
                "history": history
            }
        })

    except Exception as e:
        logger.error(f"getInterviewhistoryfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/env-status', methods=['POST'])
def get_env_status():
    """
    getsimulationenvironmentstatus

    checksimulationenvironmentwhether toalive（canreceiveInterviewcommand）

    request（JSON）：
        {
            "simulation_id": "sim_xxxx"  // Required，simulationID
        }

    Returns:
        {
            "success": true,
            "data": {
                "simulation_id": "sim_xxxx",
                "env_alive": true,
                "twitter_available": true,
                "reddit_available": true,
                "message": "environmentin progressrunning，canreceiveInterviewcommand"
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "please provide simulation_id"
            }), 400

        env_alive = SimulationRunner.check_env_alive(simulation_id)
        
        # get更detailedofstatusinformation
        env_status = SimulationRunner.get_env_status_detail(simulation_id)

        if env_alive:
            message = "environmentin progressrunning，canreceiveInterviewcommand"
        else:
            message = "environmentnotrunning or alreadyclose"

        return jsonify({
            "success": True,
            "data": {
                "simulation_id": simulation_id,
                "env_alive": env_alive,
                "twitter_available": env_status.get("twitter_available", False),
                "reddit_available": env_status.get("reddit_available", False),
                "message": message
            }
        })

    except Exception as e:
        logger.error(f"getenvironmentstatusfailed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/close-env', methods=['POST'])
def close_simulation_env():
    """
    closesimulationenvironment
    
    向simulationsendcloseenvironmentcommand，使其gracefullyexitetc待commandmode。
    
    Note:thisnot同于 /stop interface，/stop willforceterminateprocess，
    and此interfacewill让simulationgracefully地closeenvironment并exit。
    
    request（JSON）：
        {
            "simulation_id": "sim_xxxx",  // Required，simulationID
            "timeout": 30                  // Optional，timeouttime（秒），default30
        }
    
    Returns:
        {
            "success": true,
            "data": {
                "message": "environmentclosecommandalreadysend",
                "result": {...},
                "timestamp": "2025-12-08T10:00:01"
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        timeout = data.get('timeout', 30)
        
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "please provide simulation_id"
            }), 400
        
        result = SimulationRunner.close_simulation_env(
            simulation_id=simulation_id,
            timeout=timeout
        )
        
        # updatesimulation status
        manager = SimulationManager()
        state = manager.get_simulation(simulation_id)
        if state:
            state.status = SimulationStatus.COMPLETED
            manager._save_simulation_state(state)
        
        return jsonify({
            "success": result.get("success", False),
            "data": result
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
        
    except Exception as e:
        logger.error(f"Close environment failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


# ============== Panel Chat Interface ==============

@simulation_bp.route('/panel-chat', methods=['POST'])
def panel_chat():
    """
    Panel Chat - Ask all agents the same question and get aggregated responses
    
    This endpoint interviews all agents in parallel and aggregates their responses
    by stance (support/oppose/neutral) and by faction (agent type).
    
    Request (JSON):
        {
            "simulation_id": "sim_xxxx",         // Required, simulation ID
            "prompt": "What do you think about X?",  // Required, question for all agents
            "platform": "reddit",                // Optional, platform (twitter/reddit)
            "classify_stance": true,             // Optional, whether to classify stances (default true)
            "generate_summary": true,            // Optional, generate summary (default true)
            "timeout": 180                       // Optional, timeout in seconds (default 180)
        }
    
    Returns:
        {
            "success": true,
            "data": {
                "question": "What do you think about X?",
                "timestamp": "2025-12-08T10:00:01",
                "total_agents": 35,
                "stance_distribution": {"support": 45.0, "oppose": 30.0, "neutral": 25.0},
                "stance_counts": {"support": 16, "oppose": 11, "neutral": 8},
                "faction_counts": {"Student": 20, "Professor": 10, "Official": 5},
                "summary": "Overall, the panel shows mixed views...",
                "by_stance": {
                    "support": [{"agent_id": 0, "agent_name": "...", "response": "..."}],
                    "oppose": [...],
                    "neutral": [...]
                },
                "by_faction": {
                    "Student": [{"agent_id": 0, "agent_name": "...", "stance": "support"}],
                    ...
                },
                "responses": [...]  // Full list of all responses
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        prompt = data.get('prompt')
        platform = data.get('platform')
        classify_stance = data.get('classify_stance', True)
        generate_summary = data.get('generate_summary', True)
        timeout = data.get('timeout', 180)
        
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "Please provide simulation_id"
            }), 400
        
        if not prompt:
            return jsonify({
                "success": False,
                "error": "Please provide prompt (question for agents)"
            }), 400
        
        # Validate platform
        if platform and platform not in ("twitter", "reddit"):
            return jsonify({
                "success": False,
                "error": "platform must be 'twitter' or 'reddit'"
            }), 400
        
        # Load agent profiles for faction information (needed for both modes)
        manager = SimulationManager()
        profiles = manager.get_profiles(simulation_id, platform=platform or "reddit")
        
        if not profiles:
            return jsonify({
                "success": False,
                "error": "No agent profiles found for this simulation"
            }), 400
        
        # Check environment status - use profile-based LLM if env not alive
        env_alive = SimulationRunner.check_env_alive(simulation_id)
        
        if env_alive:
            # Use live interview (original behavior)
            optimized_prompt = optimize_interview_prompt(prompt)
            
            raw_result = SimulationRunner.interview_all_agents(
                simulation_id=simulation_id,
                prompt=optimized_prompt,
                platform=platform,
                timeout=timeout
            )
            
            if not raw_result.get("success", False):
                logger.error(f"Panel chat interview failed: {raw_result}")
                return jsonify({
                    "success": False,
                    "error": raw_result.get("error", "Interview failed")
                }), 500
        else:
            # Fallback: Use profile-based LLM (no live env required)
            logger.info(f"[Panel Chat] Env not alive, using profile-based LLM for {len(profiles)} agents")
            
            from app.services.agora_service import AgoraService
            agora_service = AgoraService()
            
            # Generate responses using profile-based LLM
            llm_responses = []
            for idx, profile in enumerate(profiles):
                agent_name = profile.get('username') or profile.get('display_name') or f"Agent_{idx}"
                
                try:
                    response_text = agora_service._generate_profile_response(
                        profile=profile,
                        debate_prompt=prompt,
                        system_context="You are being interviewed for a panel discussion. Answer the question directly and honestly based on your perspective and beliefs. Be concise but substantive in 2-3 sentences.",
                        agent_name=agent_name
                    )
                    
                    llm_responses.append({
                        "agent_id": idx,
                        "agent_name": agent_name,
                        "response": response_text
                    })
                except Exception as e:
                    logger.error(f"LLM response failed for agent {idx}: {e}")
            
            # Format as raw_result structure expected by aggregate_responses
            raw_result = {
                "success": True,
                "result": {
                    "responses": llm_responses
                }
            }
            logger.info(f"[Panel Chat] Generated {len(llm_responses)} profile-based LLM responses")
        
        # Aggregate responses using PanelChatService
        panel_service = PanelChatService()
        panel_result = panel_service.aggregate_responses(
            question=prompt,
            raw_results=raw_result.get("result", {}),
            profiles=profiles,
            classify_stance=classify_stance
        )
        
        # Convert to dict
        result_data = panel_result.to_dict()
        
        # Generate summary if requested
        if generate_summary:
            result_data["summary"] = panel_service.generate_summary(panel_result)
        
        return jsonify({
            "success": True,
            "data": result_data
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
    
    except TimeoutError as e:
        return jsonify({
            "success": False,
            "error": f"Panel chat timed out: {str(e)}"
        }), 504
    
    except Exception as e:
        logger.error(f"Panel chat failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


# ============== Survey Interface ==============

@simulation_bp.route('/survey/create', methods=['POST'])
def create_survey():
    """
    Create a survey template
    
    Request (JSON):
        {
            "title": "Policy Feedback Survey",
            "description": "Gather agent opinions on new policy",
            "questions": [
                {
                    "question_text": "Do you support the new policy?",
                    "question_type": "opinion_poll",
                    "options": ["Agree", "Disagree", "Neutral"]
                },
                {
                    "question_text": "How would you rate the clarity?",
                    "question_type": "likert"
                }
            ]
        }
    
    Returns:
        {
            "success": true,
            "data": {
                "survey_id": "survey_abc123",
                "title": "...",
                "questions": [...]
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        title = data.get('title')
        if not title:
            return jsonify({
                "success": False,
                "error": "Please provide survey title"
            }), 400
        
        questions = data.get('questions', [])
        if not questions:
            return jsonify({
                "success": False,
                "error": "Please provide at least one question"
            }), 400
        
        description = data.get('description', '')
        
        survey_service = SurveyService()
        template = survey_service.create_survey(
            title=title,
            questions=questions,
            description=description
        )
        
        return jsonify({
            "success": True,
            "data": template.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Create survey failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/survey/deploy', methods=['POST'])
def deploy_survey():
    """
    Deploy a survey to simulation agents
    
    Request (JSON):
        {
            "simulation_id": "sim_xxxx",
            "survey_id": "survey_abc123",
            "agent_ids": [0, 1, 2],      // Optional, specific agents to survey
            "platform": "reddit",
            "timeout": 180
        }
    
    Returns:
        {
            "success": true,
            "data": {
                "survey_id": "...",
                "total_respondents": 35,
                "aggregated": {...},
                "by_faction": {...}
            }
        }
    """
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        survey_id = data.get('survey_id')
        agent_ids = data.get('agent_ids')  # Optional: list of agent indices
        platform = data.get('platform')
        timeout = data.get('timeout', 180)
        
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "Please provide simulation_id"
            }), 400
        
        if not survey_id:
            return jsonify({
                "success": False,
                "error": "Please provide survey_id"
            }), 400
        
        # Check environment
        if not SimulationRunner.check_env_alive(simulation_id):
            return jsonify({
                "success": False,
                "error": "Simulation environment is not running"
            }), 400
        
        # Load survey template
        survey_service = SurveyService()
        template = survey_service.get_survey(survey_id)
        
        if not template:
            return jsonify({
                "success": False,
                "error": f"Survey not found: {survey_id}"
            }), 404
        
        # Build survey prompt
        survey_prompt = survey_service.build_survey_prompt(template)
        optimized_prompt = optimize_interview_prompt(survey_prompt)
        
        # Interview agents with survey
        if agent_ids and isinstance(agent_ids, list) and len(agent_ids) > 0:
            # Selective interview: only specified agents
            interviews = [{"agent_id": int(aid), "prompt": optimized_prompt} for aid in agent_ids]
            raw_result = SimulationRunner.interview_agents(
                simulation_id=simulation_id,
                interviews=interviews,
                platform=platform,
                timeout=timeout
            )
            logger.info(f"Survey deployed to {len(agent_ids)} selected agents")
        else:
            # Interview all agents (default behavior)
            raw_result = SimulationRunner.interview_all_agents(
                simulation_id=simulation_id,
                prompt=optimized_prompt,
                platform=platform,
                timeout=timeout
            )
        
        if not raw_result.get("success", False):
            return jsonify({
                "success": False,
                "error": raw_result.get("error", "Survey deployment failed"),
                "data": raw_result
            }), 500
        
        # Load profiles for faction info
        manager = SimulationManager()
        profiles = manager.get_profiles(simulation_id, platform=platform or "reddit")
        
        # Build profile lookup
        profile_lookup = {}
        for p in profiles:
            agent_id = p.get("user_id")
            if agent_id is not None:
                profile_lookup[agent_id] = p
        
        # Parse responses
        responses = []
        results_data = raw_result.get("result", {}).get("results", {})
        
        for key, result in results_data.items():
            if not isinstance(result, dict):
                continue
            
            agent_id = result.get("agent_id")
            raw_text = result.get("response", "")
            
            profile = profile_lookup.get(agent_id, {})
            agent_name = profile.get("name", f"Agent_{agent_id}")
            faction = profile.get("source_entity_type", profile.get("profession", "unknown"))
            
            parsed = survey_service.parse_agent_response(
                template=template,
                agent_id=agent_id,
                agent_name=agent_name,
                faction=faction,
                raw_response=raw_text
            )
            responses.append(parsed)
        
        # Aggregate results
        survey_result = survey_service.aggregate_results(template, responses)
        
        return jsonify({
            "success": True,
            "data": survey_result.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Deploy survey failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/survey/list', methods=['GET'])
def list_surveys():
    """List all available survey templates"""
    try:
        survey_service = SurveyService()
        surveys = survey_service.list_surveys()
        
        return jsonify({
            "success": True,
            "data": {
                "count": len(surveys),
                "surveys": [s.to_dict() for s in surveys]
            }
        })
        
    except Exception as e:
        logger.error(f"List surveys failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@simulation_bp.route('/survey/<survey_id>', methods=['GET'])
def get_survey(survey_id: str):
    """Get a specific survey template"""
    try:
        survey_service = SurveyService()
        template = survey_service.get_survey(survey_id)
        
        if not template:
            return jsonify({
                "success": False,
                "error": f"Survey not found: {survey_id}"
            }), 404
        
        return jsonify({
            "success": True,
            "data": template.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Get survey failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


# ============== Agora Debate Interface ==============

@simulation_bp.route('/agora/templates', methods=['GET'])
def get_agora_templates():
    """
    Get available debate templates (goal types)
    
    Returns:
        List of available debate goals with descriptions
    """
    templates = []
    goal_descriptions = {
        "stress_test": "Stress-test a single decision - Find weaknesses in your proposal",
        "risk_id": "Identify risks & blind spots - Uncover what you'd miss",
        "stakeholder": "Understand stakeholder perspectives - See how different groups react",
        "competitive": "Simulate competitive attacks - How would competitors respond?",
        "consensus": "Find middle ground - Identify compromise positions",
        "socratic": "Expose hidden assumptions - Deep dive into beliefs"
    }
    
    for goal_type, template in DEBATE_TEMPLATES.items():
        templates.append({
            "goal_type": goal_type,
            "name": template["name"],
            "description": goal_descriptions.get(goal_type, "")
        })
    
    return jsonify({
        "success": True,
        "data": templates
    })


@simulation_bp.route('/agora/list/<simulation_id>', methods=['GET'])
def list_agora_debates(simulation_id: str):
    """
    List all debates for a simulation
    
    Returns:
        List of debate summaries (id, topic, status, etc.)
    """
    try:
        agora_service = AgoraService()
        debates = agora_service.list_debates(simulation_id)
        
        return jsonify({
            "success": True,
            "data": debates
        })
        
    except Exception as e:
        logger.error(f"List agora debates failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@simulation_bp.route('/agora/create', methods=['POST'])
def create_agora_debate():
    """
    Create a new Agora debate
    
    Request (JSON):
        {
            "simulation_id": "sim_xxxx",
            "topic": "Should we increase minimum wage?",
            "goal_type": "stress_test",
            "agent_ids": [0, 1, 2],
            "agent_names": {"0": "Agent A", "1": "Agent B", "2": "Agent C"},
            "max_rounds": 5,
            "debate_mode": "continuous",  // or "review"
            "moderator_mode": "user_only"  // or "synthesized" or "forced_neutral"
        }
    
    Returns:
        Debate state
    """
    try:
        data = request.get_json() or {}
        
        simulation_id = data.get('simulation_id')
        if not simulation_id:
            return jsonify({
                "success": False,
                "error": "simulation_id is required"
            }), 400
        
        topic = data.get('topic')
        if not topic:
            return jsonify({
                "success": False,
                "error": "topic is required"
            }), 400
        
        agent_ids = data.get('agent_ids', [])
        if len(agent_ids) < 2:
            return jsonify({
                "success": False,
                "error": "At least 2 agents required for debate"
            }), 400
        
        # Convert agent_names keys to integers
        agent_names_raw = data.get('agent_names', {})
        agent_names = {int(k): v for k, v in agent_names_raw.items()}
        
        agora_service = AgoraService()
        state = agora_service.create_debate(
            simulation_id=simulation_id,
            topic=topic,
            goal_type=data.get('goal_type', 'stress_test'),
            agent_ids=agent_ids,
            agent_names=agent_names,
            max_rounds=data.get('max_rounds', 5),
            debate_mode=data.get('debate_mode', 'continuous'),
            moderator_mode=data.get('moderator_mode', 'user_only'),
            turn_timeout=float(data.get('turn_timeout', 60.0)),
            round_duration_seconds=int(data.get('round_duration_seconds', 30))
        )
        
        return jsonify({
            "success": True,
            "data": state.to_dict()
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
        
    except Exception as e:
        logger.error(f"Create agora debate failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/agora/<debate_id>/round', methods=['POST'])
def execute_agora_round(debate_id: str):
    """
    Execute a single debate round
    
    Request (JSON):
        {
            "pivot_topic": "Optional new topic to pivot discussion"
        }
    
    Returns:
        Round result with turns
    """
    try:
        data = request.get_json() or {}
        pivot_topic = data.get('pivot_topic')
        
        agora_service = AgoraService()
        result = agora_service.execute_round(
            debate_id=debate_id,
            pivot_topic=pivot_topic
        )
        
        return jsonify({
            "success": True,
            "data": result
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
        
    except Exception as e:
        logger.error(f"Execute agora round failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 500


@simulation_bp.route('/agora/<debate_id>/stream-round', methods=['POST'])
def stream_agora_round(debate_id: str):
    """
    Execute a timed debate round with real-time SSE streaming.
    
    This endpoint streams turn objects as they complete during the timed round,
    allowing the frontend to display the debate in real-time.
    
    Request (JSON):
        {
            "round_duration_seconds": 30,
            "pivot_topic": "Optional topic to pivot discussion"
        }
    
    Returns:
        Server-Sent Events stream of turn objects
    """
    from flask import Response
    import json as json_module
    
    # Extract request data BEFORE entering the generator (Flask context issue)
    data = request.get_json() or {}
    round_duration = data.get('round_duration_seconds', 30)
    pivot_topic = data.get('pivot_topic')
    
    def generate():
        try:
            agora_service = AgoraService()
            
            # Stream turns as they complete
            for turn_data in agora_service.execute_timed_round(
                debate_id=debate_id,
                round_duration_seconds=round_duration,
                pivot_topic=pivot_topic
            ):
                # Format as SSE
                yield f"data: {json_module.dumps(turn_data)}\n\n"
                
        except ValueError as e:
            yield f"data: {json_module.dumps({'_type': 'error', 'error': str(e)})}\n\n"
            
        except Exception as e:
            logger.error(f"Stream agora round failed: {str(e)}")
            yield f"data: {json_module.dumps({'_type': 'error', 'error': str(e)})}\n\n"
    
    return Response(
        generate(),
        mimetype='text/event-stream',
        headers={
            'Cache-Control': 'no-cache',
            'Connection': 'keep-alive',
            'X-Accel-Buffering': 'no'
        }
    )

@simulation_bp.route('/agora/<debate_id>', methods=['GET'])
def get_agora_debate_by_id(debate_id: str):
    """Get full debate state by ID"""
    try:
        agora_service = AgoraService()
        state = agora_service.get_debate(debate_id)
        
        if not state:
            return jsonify({
                "success": False,
                "error": f"Debate not found: {debate_id}"
            }), 404
        
        return jsonify({
            "success": True,
            "data": state.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Get agora debate failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@simulation_bp.route('/agora/<debate_id>/status', methods=['GET'])
def get_agora_status(debate_id: str):
    """Get current debate status"""
    try:
        agora_service = AgoraService()
        state = agora_service.get_debate(debate_id)
        
        if not state:
            return jsonify({
                "success": False,
                "error": f"Debate not found: {debate_id}"
            }), 404
        
        return jsonify({
            "success": True,
            "data": state.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Get agora status failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@simulation_bp.route('/agora/<debate_id>/pause', methods=['POST'])
def pause_agora_debate(debate_id: str):
    """Pause a running debate"""
    try:
        agora_service = AgoraService()
        state = agora_service.pause_debate(debate_id)
        
        return jsonify({
            "success": True,
            "data": state.to_dict()
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
        
    except Exception as e:
        logger.error(f"Pause agora debate failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@simulation_bp.route('/agora/<debate_id>/resume', methods=['POST'])
def resume_agora_debate(debate_id: str):
    """Resume a paused debate"""
    try:
        agora_service = AgoraService()
        state = agora_service.resume_debate(debate_id)
        
        return jsonify({
            "success": True,
            "data": state.to_dict()
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
        
    except Exception as e:
        logger.error(f"Resume agora debate failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@simulation_bp.route('/agora/<debate_id>/stop', methods=['POST'])
def stop_agora_debate(debate_id: str):
    """
    Stop a debate permanently and generate summary
    
    Request (JSON):
        {
            "generate_summary": true  // Optional, default true
        }
    """
    try:
        data = request.get_json() or {}
        generate_summary = data.get('generate_summary', True)
        
        agora_service = AgoraService()
        state = agora_service.stop_debate(
            debate_id=debate_id,
            generate_summary=generate_summary
        )
        
        return jsonify({
            "success": True,
            "data": state.to_dict()
        })
        
    except ValueError as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 400
        
    except Exception as e:
        logger.error(f"Stop agora debate failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500
