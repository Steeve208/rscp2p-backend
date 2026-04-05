"""
Schemas para datos de mercado (OHLC para gráficos).
"""

from pydantic import BaseModel


class OHLCCandle(BaseModel):
    time: int  # Unix timestamp (segundos)
    open: float
    high: float
    low: float
    close: float
    volume: float = 0.0
