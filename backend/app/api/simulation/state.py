
from flask import Blueprint, request, jsonify
from ...services.simulation_manager import SimulationManager
from ...services.simulation_runner import SimulationRunner
from ...services.neo4j_entity_reader import Neo4jEntityReader
from ...utils.logger import get_logger

logger = get_logger('pubop.api.simulation.state')
state_bp = Blueprint('simulation_state', __name__)

@state_bp.route('/<simulation_id>', methods=['GET'])
def get_simulation(simulation_id: str):
    try:
        manager = SimulationManager()
        sim = manager.get_simulation(simulation_id)
        if not sim:
             return jsonify({"success": False, "error": "Not found"}), 404
             
        # Try to get runtime status if available
        try:
             runner = SimulationRunner()
             run_state = runner.get_run_state(simulation_id)
             if run_state:
                 sim.status = run_state.runner_status # Update status from runner
        except:
             pass
             
        return jsonify({"success": True, "data": sim.to_dict()})
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500

@state_bp.route('/<simulation_id>/tree', methods=['GET'])
def get_simulation_tree(simulation_id: str):
    try:
        runner = SimulationRunner()
        lineage = runner.get_simulation_lineage(simulation_id)
        return jsonify({"success": True, "data": lineage})
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500

@state_bp.route('/entities/<graph_id>', methods=['GET'])
def get_graph_entities(graph_id: str):
    try:
        entity_types = request.args.get('entity_types', '').split(',')
        entity_types = [t.strip() for t in entity_types if t.strip()]
        enrich = request.args.get('enrich', 'true') == 'true'
        
        reader = Neo4jEntityReader()
        result = reader.filter_defined_entities(
            graph_id=graph_id, 
            defined_entity_types=entity_types or None,
            enrich_with_edges=enrich
        )
        return jsonify({"success": True, "data": result.to_dict()})
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500
