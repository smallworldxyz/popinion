"""
API Decorators for Popinion Backend
Provides reusable decorators for common route patterns.
"""

from functools import wraps
from flask import request, jsonify
from pydantic import ValidationError
from typing import Type, TypeVar, Callable
from dataclasses import asdict

from .services.base_service import Result
from .utils.logger import get_logger

logger = get_logger('pubop.decorators')

T = TypeVar('T')


def validate(schema_class: Type[T]) -> Callable:
    """
    Decorator to validate request body against a Pydantic schema.
    
    Usage:
        @routes_bp.route('/create', methods=['POST'])
        @login_required
        @validate(MyRequestSchema)
        def create_resource(validated: MyRequestSchema):
            # validated is the parsed Pydantic model
            return {"success": True, "data": validated.dict()}
    
    Args:
        schema_class: Pydantic model class to validate against
        
    Returns:
        Decorator function
    """
    def decorator(f: Callable) -> Callable:
        @wraps(f)
        def wrapper(*args, **kwargs):
            try:
                # Parse and validate request body
                body = request.get_json(silent=True) or {}
                validated = schema_class(**body)
                
                # Call the original function with validated schema as first arg
                return f(validated, *args, **kwargs)
                
            except ValidationError as e:
                # Return structured validation error
                return jsonify({
                    "success": False,
                    "error": "Validation error",
                    "code": "validation_error",
                    "details": e.errors()
                }), 400
                
        return wrapper
    return decorator


def result_response(f: Callable) -> Callable:
    """
    Decorator to automatically convert Result[T] returns to Flask responses.
    
    Usage:
        @routes_bp.route('/get/<id>')
        @result_response
        def get_resource(id: str):
            service = get_service()
            return service.get_by_id(id)  # Returns Result[T]
    
    The decorator will:
    - Convert Result.success to {"success": True, "data": ...}
    - Convert Result.failure to {"success": False, "error": ..., "code": ...}
    """
    @wraps(f)
    def wrapper(*args, **kwargs):
        result = f(*args, **kwargs)
        
        # If not a Result object, return as-is (for backward compatibility)
        if not isinstance(result, Result):
            return result
        
        if result.ok:
            value = result.value
            # Convert dataclass to dict if needed
            if hasattr(value, '__dataclass_fields__'):
                data = asdict(value)
            elif isinstance(value, dict):
                data = value
            elif isinstance(value, list):
                # Handle list of dataclasses
                data = [
                    asdict(item) if hasattr(item, '__dataclass_fields__') else item
                    for item in value
                ]
            else:
                data = value
                
            return jsonify({"success": True, "data": data})
        else:
            # Map error code to HTTP status
            status_map = {
                "not_found": 404,
                "validation_error": 400,
                "internal_error": 500,
                "unauthorized": 401,
                "forbidden": 403
            }
            code_value = result.error.code.value if hasattr(result.error.code, 'value') else str(result.error.code)
            status = status_map.get(code_value, 500)
            
            return jsonify({
                "success": False,
                "error": result.error.message,
                "code": code_value,
                "details": result.error.details
            }), status
            
    return wrapper


def validate_and_result(schema_class: Type[T]) -> Callable:
    """
    Combined decorator: validates input AND converts Result output.
    
    Usage:
        @routes_bp.route('/create', methods=['POST'])
        @login_required
        @validate_and_result(CreateRequest)
        def create(req: CreateRequest):
            service = get_service()
            return service.create(req)  # Returns Result[T]
    """
    def decorator(f: Callable) -> Callable:
        # Apply both decorators in order
        validated_f = validate(schema_class)(f)
        result_f = result_response(validated_f)
        return result_f
    return decorator
