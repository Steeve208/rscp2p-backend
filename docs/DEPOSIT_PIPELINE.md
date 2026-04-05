# Deterministic Deposit Processing Pipeline

Deposits from blockchain watchers are processed **exactly once** in a single database transaction.

## Flow

```
BlockchainWatcher
   → DepositEvent (payload)
   → Idempotency Guard (deposit_events table)
   → Order Aggregate (row-level lock)
   → Ledger Entry (double-entry)
   → Escrow State Update
   → Domain Event (outbox)
   → COMMIT
```

## 1. Event schema

**DepositEventPayload** (incoming from watcher):

| Field               | Type   | Description                    |
|---------------------|--------|--------------------------------|
| order_id            | str    | Order to fund                  |
| tx_hash             | str    | Blockchain transaction hash   |
| amount              | str    | Deposit amount (exact match)   |
| currency            | str    | e.g. USDT                      |
| external_escrow_id  | str    | External escrow reference     |
| contract_address    | str    | Contract address               |
| idempotency_key     | str?   | Optional; else `deposit:{order_id}:{tx_hash}` |

**DepositResult** (returned):

| Field             | Type | Description                          |
|-------------------|------|--------------------------------------|
| order_id          | str  | Order id                              |
| order_status      | str  | e.g. ESCROW_FUNDED                    |
| escrow_funded     | bool | True if escrow was funded             |
| already_processed | bool | True if duplicate (cached result)     |
| rejected          | bool | True if event was rejected            |
| rejection_reason | str? | Reason when rejected                  |

## 2. Tables

### deposit_events

Append-only event log and idempotency store. One row per logical event (unique `idempotency_key`).

| Column             | Type   | Description                    |
|--------------------|--------|--------------------------------|
| id                 | PK     | UUID                           |
| idempotency_key    | UNIQUE | Claim key                      |
| order_id           |        | Order id                       |
| tx_hash, amount, currency, external_escrow_id, contract_address | | Event payload |
| status             |        | PENDING → PROCESSED \| REJECTED |
| result_snapshot    |        | Order status when processed   |
| rejection_reason   |        | Set when REJECTED              |
| processed_at       |        | When status was set            |
| created_at         |        | Insert time                    |

### idempotency_keys

Existing table; used by other flows. Deposit pipeline uses **deposit_events** as the idempotency guard.

## 3. Transaction boundaries (single transaction)

```
BEGIN
  1. INSERT deposit_events (idempotency_key, ..., status=PENDING)
     → If duplicate key: ROLLBACK, return cached result (no lock).
  2. SELECT order FOR UPDATE (row-level lock).
  3. If order not found: UPDATE deposit_events SET status=REJECTED; COMMIT; raise 404.
  4. Validate amount (partial / overpayment → REJECTED); COMMIT; raise 409 if invalid.
  5. Apply ledger entries (double-entry: buyer_balance -amount, escrow +amount).
  6. Order aggregate: apply_deposit (escrow state update).
  7. Persist domain events (domain_events + outbox_events).
  8. UPDATE deposit_events SET status=PROCESSED, result_snapshot, processed_at.
COMMIT
```

All steps run in one transaction; commit is atomic.

## 4. Concurrency safeguards

- **Idempotency**: First request to insert `deposit_events` with a given `idempotency_key` wins. Second and later get `IntegrityError` (unique), then return cached result from existing row.
- **Row-level lock**: `get_for_update(order_id)` ensures only one transaction mutates the order at a time.
- **Determinism**: Same payload + same key always yields same result; duplicates never re-apply.

## 5. Validation (partial / overpayment)

- **Partial deposit** (amount < order amount): Rejected; `deposit_events.status = REJECTED`, `rejection_reason = "Partial deposit not allowed"`.
- **Overpayment** (amount > order amount): Rejected; `rejection_reason = "Overpayment not allowed"`.
- **Exact match** (amount == order amount, currency match): Processed.

## 6. Tests

- **test_deposit_processor.py**: Duplicate events (cached), same-key multiple calls (one applies), partial rejected, overpayment rejected, transaction boundaries (PROCESSED + result_snapshot), order not found (REJECTED + 404).
