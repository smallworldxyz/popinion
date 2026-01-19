from flask import Blueprint, jsonify, request
from ..models.project import ProjectManager
from ..auth import login_required
from ..utils.logger import get_logger

project_bp = Blueprint('project', __name__)
logger = get_logger('pubop.api.project')

@project_bp.route('/', methods=['GET'])
@login_required
def list_projects():
    """List all projects with metadata"""
    try:
        limit = int(request.args.get('limit', 50))
        projects = ProjectManager.list_projects(limit=limit)
        return jsonify({
            "success": True,
            "data": [p.to_dict() for p in projects]
        })
    except Exception as e:
        logger.error(f"List projects failed: {str(e)}")
        return jsonify({"success": False, "error": str(e)}), 500

@project_bp.route('/<project_id>', methods=['GET'])
@login_required
def get_project(project_id):
    """Get single project detail"""
    try:
        project = ProjectManager.get_project(project_id)
        if not project:
            return jsonify({"success": False, "error": "Project not found"}), 404
        return jsonify({
            "success": True,
            "data": project.to_dict()
        })
    except Exception as e:
        logger.error(f"Get project failed: {str(e)}")
        return jsonify({"success": False, "error": str(e)}), 500

@project_bp.route('/<project_id>', methods=['DELETE'])
@login_required
def delete_project(project_id):
    """Delete a project"""
    try:
        success = ProjectManager.delete_project(project_id)
        if not success:
            return jsonify({"success": False, "error": "Project not found or delete failed"}), 404
        return jsonify({
            "success": True, 
            "message": f"Project {project_id} deleted"
        })
    except Exception as e:
        logger.error(f"Delete project failed: {str(e)}")
        return jsonify({"success": False, "error": str(e)}), 500
