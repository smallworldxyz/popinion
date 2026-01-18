from flask import Blueprint, request, jsonify
import os
import json
from ..utils.logger import get_logger

annotation_bp = Blueprint('annotation', __name__, url_prefix='/api/simulation')
logger = get_logger('pubop.annotation')

def _get_annotation_file_path(simulation_id):
    # Retrieve the simulation directory from the uploads folder
    # Assuming standard path structure: backend/uploads/simulations/<id>/annotations.json
    base_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    sim_dir = os.path.join(base_dir, 'uploads', 'simulations', simulation_id)
    os.makedirs(sim_dir, exist_ok=True)
    return os.path.join(sim_dir, 'annotations.json')

def _load_annotations(simulation_id):
    path = _get_annotation_file_path(simulation_id)
    if not os.path.exists(path):
        return {}
    try:
        with open(path, 'r', encoding='utf-8') as f:
            return json.load(f)
    except Exception as e:
        logger.error(f"Failed to load annotations for {simulation_id}: {e}")
        return {}

def _save_annotations(simulation_id, annotations):
    path = _get_annotation_file_path(simulation_id)
    try:
        with open(path, 'w', encoding='utf-8') as f:
            json.dump(annotations, f, ensure_ascii=False, indent=2)
        return True
    except Exception as e:
        logger.error(f"Failed to save annotations for {simulation_id}: {e}")
        return False

@annotation_bp.route('/<simulation_id>/annotations', methods=['GET'])
def get_annotations(simulation_id):
    """Get all annotations for a simulation"""
    annotations = _load_annotations(simulation_id)
    return jsonify({
        "success": True,
        "data": annotations
    })

@annotation_bp.route('/<simulation_id>/annotate', methods=['POST'])
def add_annotation(simulation_id):
    """
    Add or update an annotation for a specific action/round
    Request: { "action_id": "...", "content": "...", "author": "..." }
    """
    try:
        data = request.get_json() or {}
        action_id = data.get('action_id')
        content = data.get('content')
        author = data.get('author', 'User')  # Could be 'Director'

        if not action_id or not content:
            return jsonify({"success": False, "error": "Missing action_id or content"}), 400

        annotations = _load_annotations(simulation_id)
        
        # Structure: { action_id: { content, author, timestamp } }
        from datetime import datetime
        annotations[action_id] = {
            "content": content,
            "author": author,
            "timestamp": datetime.now().isoformat()
        }

        if _save_annotations(simulation_id, annotations):
            return jsonify({"success": True, "data": annotations[action_id]})
        else:
            return jsonify({"success": False, "error": "Failed to save"}), 500

    except Exception as e:
        logger.error(f"Annotation error: {e}")
        return jsonify({"success": False, "error": str(e)}), 500

@annotation_bp.route('/<simulation_id>/annotate/<action_id>', methods=['DELETE'])
def delete_annotation(simulation_id, action_id):
    """Delete an annotation"""
    try:
        annotations = _load_annotations(simulation_id)
        if action_id in annotations:
            del annotations[action_id]
            _save_annotations(simulation_id, annotations)
        
        return jsonify({"success": True})
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500
