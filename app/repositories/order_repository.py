"""
Repositorio del agregado Order. Carga con bloqueo y persiste sin commit.
"""

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.domain.order_aggregate import OrderAggregate
from app.models.marketplace import EscrowModel, OrderModel


class OrderRepository:
    """Carga y persiste OrderAggregate en la misma sesión. Commit lo hace el caller."""

    def __init__(self, db: Session):
        self._db = db

    def get_for_update(self, order_id: str) -> OrderAggregate | None:
        """
        Carga order + escrow con SELECT FOR UPDATE en la misma sesión.
        Retorna OrderAggregate o None si la orden no existe.
        """
        order = self._db.scalar(
            select(OrderModel).where(OrderModel.id == order_id).with_for_update()
        )
        if order is None:
            return None
        escrow = self._db.scalar(
            select(EscrowModel).where(EscrowModel.order_id == order_id).with_for_update()
        )
        return OrderAggregate(order, escrow)

    def save(self, aggregate: OrderAggregate) -> None:
        """
        Asegura persistencia del aggregate (order + escrow) en la sesión.
        No hace commit; el caller debe hacer db.commit().
        """
        self._db.add(aggregate.order)
        if aggregate.escrow is not None:
            self._db.add(aggregate.escrow)
        self._db.flush()
