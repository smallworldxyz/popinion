from flask import Blueprint, request, jsonify
from ...auth import login_required
from ...services.ingestion_service import IngestionService
import asyncio

ingestion_bp = Blueprint('ingestion', __name__)

@ingestion_bp.route('/start', methods=['POST'])
@login_required
def start_ingestion():
    """
    Start an ad-hoc ingestion job.
    payload: {
        "sources": ["https://site/rss", "telegram_channel"],
        "keywords": ["politics", "economy"],
        "limit": 10
    }
    """
    data = request.get_json() or {}
    sources = data.get('sources')
    keywords = data.get('keywords')
    limit = data.get('limit', 10)
    
    # Run async in background (simple approach) or await if short
    # Since flask is threaded, we can run async loop here or offload.
    # For MVP, we'll await it (blocking req) to show results immediately.
    # In prod, use Celery.
    
    try:
        result = asyncio.run(IngestionService.run_now(
            sources=sources,
            keywords=keywords,
            limit_per_source=limit
        ))
        return jsonify({"success": True, "data": result})
    except Exception as e:
        return jsonify({"success": False, "error": str(e)}), 500

@ingestion_bp.route('/sources/defaults', methods=['GET'])
@login_required
def get_defaults():
    """Get default RSS sources list"""
    return jsonify({
        "success": True, 
        "data": IngestionService.DEFAULT_RSS_SOURCES
    })
