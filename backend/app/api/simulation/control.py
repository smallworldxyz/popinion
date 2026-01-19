
import os
from flask import Blueprint, request, jsonify
from ...services.simulation_runner import SimulationRunner, RunnerStatus
from ...services.simulation_ipc import SimulationIPCClient
from ...config import Config
from ...auth import login_required
from ...utils.logger import get_logger

logger = get_logger('pubop.api.simulation.control')
control_bp = Blueprint('simulation_control', __name__)

@control_bp.route('/<simulation_id>/pause', methods=['POST'])
@login_required
def pause_simulation(simulation_id: str):
    try:
        runner = SimulationRunner()
        if runner.pause_simulation(simulation_id):
            return jsonify({"success": True, "status": "paused"})
        return jsonify({"success": False, "error": "Failed to pause"}), 500
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500

@control_bp.route('/<simulation_id>/resume', methods=['POST'])
@login_required
def resume_simulation(simulation_id: str):
    try:
        runner = SimulationRunner()
        if runner.resume_simulation(simulation_id):
            return jsonify({"success": True, "status": "running"})
        return jsonify({"success": False, "error": "Failed to resume"}), 500
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500

@control_bp.route('/<simulation_id>/inject', methods=['POST'])
@login_required
def inject_event(simulation_id: str):
    try:
        data = request.get_json() or {}
        event_text = data.get('event_text')
        
        sim_dir = os.path.join(Config.OASIS_SIMULATION_DATA_DIR, simulation_id)
        client = SimulationIPCClient(sim_dir)
        
        if not client.check_env_alive():
             return jsonify({"success": False, "error": "Simulation not running"}), 400
             
        response = client.send_inject_event(event_text)
        return jsonify({"success": True, "data": response.to_dict()})
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500

@control_bp.route('/<simulation_id>/fork', methods=['POST'])
@login_required
def fork_simulation(simulation_id: str):
    try:
        data = request.get_json() or {}
        round_num = data.get('round')
        if not round_num:
             return jsonify({"success": False, "error": "round required"}), 400
             
        runner = SimulationRunner()
        new_id = runner.fork_simulation(simulation_id, round_num)
        return jsonify({
            "success": True, 
            "data": {"new_simulation_id": new_id, "fork_round": round_num}
        })
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500
