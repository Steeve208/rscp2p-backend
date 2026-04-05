"""
Respuesta paginada genérica (alineada con frontend).
"""

from typing import Generic, TypeVar

from pydantic import BaseModel

T = TypeVar("T")


class PaginatedResponse(BaseModel, Generic[T]):
    data: list[T]
    total: int
    page: int
    limit: int
    totalPages: int
