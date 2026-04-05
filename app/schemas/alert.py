"""
Schemas para alertas del terminal (alineados con terminalAlerts.store).
"""

from typing import Any, Literal

from pydantic import BaseModel, Field

AlertType = Literal[
    "whale-movement",
    "manipulation-detected",
    "price-threshold",
    "volume-spike",
]
Severity = Literal["low", "medium", "high", "critical"]


class Alert(BaseModel):
    id: str
    type: AlertType
    title: str
    message: str
    severity: Severity
    timestamp: int
    read: bool = False
    data: dict[str, Any] | None = None


class AlertCreate(BaseModel):
    type: AlertType
    title: str
    message: str
    severity: Severity = "medium"
    data: dict[str, Any] | None = None
