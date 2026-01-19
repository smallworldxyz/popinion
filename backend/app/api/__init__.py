from flask import Blueprint

# Define Blueprints
graph_bp = Blueprint('graph', __name__)
report_bp = Blueprint('report', __name__)
crawl_bp = Blueprint('crawl', __name__)
graph_fusion_bp = Blueprint('graph_fusion', __name__)
project_bp = Blueprint('project', __name__)

# Import views to register routes
from . import graph, report, tools
from .crawl import crawl_bp
from .graph_fusion import graph_fusion_bp
from .project import project_bp

# Import refactored simulation blueprint
# This comes from backend/app/api/simulation/__init__.py
from .simulation import simulation_bp
