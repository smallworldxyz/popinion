from flask import request, jsonify
from . import graph_fusion_bp
from ..services.graph_merge_service import GraphMergeService, MergeConfig
from ..utils.logger import get_logger

logger = get_logger('pubop.api.graph_fusion')

@graph_fusion_bp.route('/preview', methods=['POST'])
def preview_merge():
    """
    Preview graph merge results
    
    Request:
        {
            "source_graph_id": "uuid",
            "target_graph_id": "uuid"
        }
        
    Returns:
        EntityOverlapResult JSON
    """
    try:
        data = request.get_json()
        if not data:
            return jsonify({"success": False, "error": "No JSON data provided"}), 400
            
        source_id = data.get('source_graph_id')
        target_id = data.get('target_graph_id')
        
        if not source_id or not target_id:
            return jsonify({"success": False, "error": "source_graph_id and target_graph_id are required"}), 400
            
        service = GraphMergeService()
        result = service.detect_entity_overlaps(source_id, target_id)
        
        return jsonify({
            "success": True,
            "data": result.__dict__
        })
        
    except Exception as e:
        logger.error(f"Merge preview failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500

@graph_fusion_bp.route('/execute', methods=['POST'])
def execute_merge():
    """
    Execute graph merge
    """
    try:
        data = request.get_json()
        source_id = data.get('source_graph_id')
        target_id = data.get('target_graph_id')
        strategy = data.get('strategy', 'target_authoritative')
        
        if not source_id or not target_id:
            return jsonify({"success": False, "error": "source_graph_id and target_graph_id are required"}), 400
            
        config = MergeConfig(merge_strategy=strategy)
        service = GraphMergeService()
        result = service.merge_graphs(source_id, target_id, config)
        
        return jsonify({
            "success": True,
            "data": result
        })
        
    except Exception as e:
        logger.error(f"Merge execution failed: {str(e)}")
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500
