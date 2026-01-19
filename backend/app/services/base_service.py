"""
Base service class for consistent patterns
Following backend-dev-guidelines
"""

from typing import TypeVar, Generic, Optional, Any, Dict
from dataclasses import dataclass
from enum import Enum
from ..utils.logger import get_logger

T = TypeVar("T")
E = TypeVar("E")


class ErrorCode(str, Enum):
    NOT_FOUND = "not_found"
    VALIDATION_ERROR = "validation_error"
    INTERNAL_ERROR = "internal_error"
    UNAUTHORIZED = "unauthorized"
    FORBIDDEN = "forbidden"


@dataclass
class ServiceError:
    code: ErrorCode
    message: str
    details: Optional[Dict[str, Any]] = None


@dataclass
class Result(Generic[T]):
    """Result type for service methods - error or success"""
    ok: bool
    value: Optional[T] = None
    error: Optional[ServiceError] = None
    
    @classmethod
    def success(cls, value: T) -> "Result[T]":
        return cls(ok=True, value=value)
    
    @classmethod
    def failure(cls, code: ErrorCode, message: str, details: Dict = None) -> "Result[T]":
        return cls(ok=False, error=ServiceError(code=code, message=message, details=details))


class BaseService:
    """Base class for all services - provides common patterns"""
    
    def __init__(self, name: str = "service"):
        self.logger = get_logger(f"pubop.services.{name}")
    
    def handle_error(self, error: Exception, operation: str) -> ServiceError:
        """Log and convert exception to ServiceError"""
        self.logger.error(f"{operation} failed: {str(error)}")
        return ServiceError(
            code=ErrorCode.INTERNAL_ERROR,
            message=str(error)
        )
    
    def validate_required(self, data: Dict, fields: list) -> Optional[ServiceError]:
        """Validate required fields exist"""
        missing = [f for f in fields if f not in data or data[f] is None]
        if missing:
            return ServiceError(
                code=ErrorCode.VALIDATION_ERROR,
                message=f"Missing required fields: {', '.join(missing)}"
            )
        return None
