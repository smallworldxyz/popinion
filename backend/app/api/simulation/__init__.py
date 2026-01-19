
from flask import Blueprint
from .routes import routes_bp
from .state import state_bp
from .control import control_bp

# Main Blueprint
simulation_bp = Blueprint('simulation', __name__)

# Register Sub-blueprints
# Note: creating a nested structure
# /api/simulation/create -> routes_bp
# /api/simulation/:id -> state_bp (mostly)

# We can register them directly to the main app, or nest them.
# To maintain backward compatibility with the existing frontend:
# The frontend uses /api/simulation/create
# /api/simulation/:id/pause

# Let's register routes_bp with no prefix (relative to /api/simulation)
simulation_bp.register_blueprint(routes_bp)
simulation_bp.register_blueprint(state_bp)
simulation_bp.register_blueprint(control_bp)
