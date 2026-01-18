from flask import Blueprint

# Define Blueprints
graph_bp = Blueprint('graph', __name__)
simulation_bp = Blueprint('simulation', __name__)
report_bp = Blueprint('report', __name__)
crawl_bp = Blueprint('crawl', __name__)
graph_fusion_bp = Blueprint('graph_fusion', __name__)

# Import views to register routes
from . import graph, simulation, report, crawl, graph_fusion, tools
