from flask import Blueprint, request, jsonify
from ..auth import create_access_token, MOCK_USERS, User
from ..config import Config

auth_bp = Blueprint('auth', __name__)

@auth_bp.route('/login', methods=['POST'])
def login():
    data = request.get_json()
    username = data.get('username')
    password = data.get('password')
    
    # Simple Mock Login for now (Strict in Server Mode)
    # Password check is bypassed for mock users in this MVP phase
    user = MOCK_USERS.get(username)
    if not user:
         # In Desktop Mode, auto-register "Owner"?
         # For SaaS, return 401
         return jsonify({"error": "Invalid credentials"}), 401
         
    # Generate Token
    access_token = create_access_token(data={"sub": user.id})
    return jsonify({
        "access_token": access_token, 
        "token_type": "bearer",
        "user": user.dict()
    })

@auth_bp.route('/register', methods=['POST'])
def register():
    # Placeholder for Registration logic
    # In Desktop Mode, this just creates a profile.
    return jsonify({"message": "Registration not implemented yet"}), 501

@auth_bp.route('/me', methods=['GET'])
def me():
    # This endpoint relies on the @login_required decorator which injects 'g.current_user'
    # We need to import it to use it here, or just handle manually if circular import
    from ..auth import login_required, get_current_user
    
    @login_required
    def _me():
        user = get_current_user()
        if not user:
             return jsonify({"error": "Not authenticated"}), 401
        return jsonify(user.dict())
        
    return _me()
