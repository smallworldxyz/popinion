import os
import jwt
import datetime
from typing import Optional
from functools import wraps
from flask import request, jsonify, g
from pydantic import BaseModel
from .config import Config

# Models
class User(BaseModel):
    id: str
    username: str
    email: Optional[str] = None
    role: str = "user" # user, admin
    api_key: Optional[str] = None # BYOK

class Token(BaseModel):
    access_token: str
    token_type: str

# Mock Database (InMemory for now, or SQLite in future)
# For Desktop Mode, we have one persistent "Owner" user.
MOCK_USERS = {
    "owner": User(id="owner", username="Owner", role="admin"),
    "admin@popinion.com": User(id="admin", username="Admin", email="admin@popinion.com", role="admin")
}

SECRET_KEY = Config.SECRET_KEY
ALGORITHM = "HS256"

def create_access_token(data: dict, expires_delta: Optional[datetime.timedelta] = None):
    to_encode = data.copy()
    if expires_delta:
        expire = datetime.datetime.utcnow() + expires_delta
    else:
        expire = datetime.datetime.utcnow() + datetime.timedelta(minutes=15)
    to_encode.update({"exp": expire})
    encoded_jwt = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    return encoded_jwt

def verify_token(token: str):
    try:
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        username: str = payload.get("sub")
        if username is None:
            return None
        return username
    except jwt.PyJWTError:
        return None

# Dependency / Decorator
def login_required(f):
    @wraps(f)
    def decorated_function(*args, **kwargs):
        # HYBRID LOGIC:
        # If in DESKTOP_MODE, we can be more relaxed or auto-login.
        # But for consistency, the Frontend in Electron should still send a "Desktop Token".
        
        if Config.DESKTOP_MODE:
            # In Desktop Mode, if no token is present, we assume it's the Owner.
            # This allows CLI or simple usage without login.
            # However, if the frontend sends a token, we verified it.
            token = None
            if 'Authorization' in request.headers:
                auth_header = request.headers['Authorization']
                if auth_header.startswith("Bearer "):
                    token = auth_header.split(" ")[1]
            
            if not token:
                # Auto-Context for Desktop
                g.current_user = MOCK_USERS["owner"]
                return f(*args, **kwargs)
        
        # Server Mode (or Desktop with Token)
        token = None
        if 'Authorization' in request.headers:
            auth_header = request.headers['Authorization']
            if auth_header.startswith("Bearer "):
                token = auth_header.split(" ")[1]
        
        if not token:
            return jsonify({"error": "Authentication required"}), 401
            
        username = verify_token(token)
        if not username:
             return jsonify({"error": "Invalid token"}), 401
             
        # In a real DB, fetch user. MOCK for now
        # If username matches a key in MOCK, use it. Else create generic user context.
        user = MOCK_USERS.get(username)
        if not user:
            # Dynamic user for SaaS simulation
             user = User(id=username, username=username)
             
        g.current_user = user
        return f(*args, **kwargs)
        
    return decorated_function

def get_current_user():
    return getattr(g, 'current_user', None)
