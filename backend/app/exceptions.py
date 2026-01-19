"""
Custom Exceptions for Popinion Backend
Following backend-dev-guidelines for consistent error handling.
"""

from typing import Optional, Dict, Any


class PopinionError(Exception):
    """Base exception for all Popinion errors"""
    
    def __init__(self, message: str, code: str = "internal_error", details: Dict[str, Any] = None):
        super().__init__(message)
        self.message = message
        self.code = code
        self.details = details or {}
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "error": self.code,
            "message": self.message,
            "details": self.details
        }


class NotFoundError(PopinionError):
    """Resource not found"""
    
    def __init__(self, resource: str, identifier: str):
        super().__init__(
            message=f"{resource} not found: {identifier}",
            code="not_found",
            details={"resource": resource, "identifier": identifier}
        )


class ValidationError(PopinionError):
    """Input validation failed"""
    
    def __init__(self, message: str, field: str = None, details: Dict = None):
        super().__init__(
            message=message,
            code="validation_error",
            details={"field": field, **(details or {})}
        )


class ConfigurationError(PopinionError):
    """System misconfiguration"""
    
    def __init__(self, message: str, component: str = None):
        super().__init__(
            message=message,
            code="configuration_error",
            details={"component": component}
        )


class ExternalServiceError(PopinionError):
    """External service (LLM, Neo4j, etc.) failed"""
    
    def __init__(self, service: str, message: str, original_error: Exception = None):
        super().__init__(
            message=f"{service} error: {message}",
            code="external_service_error",
            details={
                "service": service,
                "original_error": str(original_error) if original_error else None
            }
        )


class SimulationError(PopinionError):
    """Simulation-specific error"""
    
    def __init__(self, simulation_id: str, message: str, stage: str = None):
        super().__init__(
            message=message,
            code="simulation_error",
            details={"simulation_id": simulation_id, "stage": stage}
        )


class PermissionError(PopinionError):
    """Insufficient permissions"""
    
    def __init__(self, action: str, resource: str):
        super().__init__(
            message=f"Permission denied: {action} on {resource}",
            code="permission_denied",
            details={"action": action, "resource": resource}
        )
