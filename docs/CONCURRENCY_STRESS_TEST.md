# Concurrency stress test: deposit processing

## Test: `test_concurrency_stress_20_workers_same_order_one_deposit_applied`

### Setup

- **20 concurrent workers** (thread pool), each with its own database session from a **shared engine**.
- All workers process the **same deposit** for the **same order** (same `order_id`, same `idempotency_key`, same `tx_hash`, same amount).
- Database: file-backed SQLite so all connections see the same data (in-memory SQLite would not be shared across threads).

### Expected behavior

1. **Idempotency (exactly-once)**  
   The table `deposit_events` has a **UNIQUE** constraint on `idempotency_key`. Only the first worker that successfully **INSERT**s a row wins; the other 19 get **IntegrityError** (duplicate key), rollback, and return the **cached result** (`already_processed=True`) without applying the deposit again.

2. **Single application of the deposit**  
   Exactly **one** worker completes the full pipeline:
   - Lock order row (`SELECT ... FOR UPDATE`)
   - Validate amount
   - Insert **ledger entries** (double-entry: buyer_balance −amount, escrow +amount)
   - Update **escrow state** (create/update escrow, order → ESCROW_FUNDED)
   - Persist domain events
   - Update `deposit_events` to PROCESSED and commit

3. **No duplicate ledger entries**  
   Only one deposit is applied, so there are exactly **2 ledger rows** for that order (one deposit = 2 entries: debit buyer_balance, credit escrow). Any duplicate application would add more rows and break the invariant.

### Invariants asserted after all workers finish

| Invariant | Assertion |
|-----------|-----------|
| Order state | `order.status == "ESCROW_FUNDED"` |
| Escrow ledger balance = order amount | `get_balance(order_id, ACCOUNT_ESCROW, currency) == Decimal(order_amount)` |
| No duplicate ledger entries | `count(ledger_entries WHERE order_id = X) == 2` |
| Exactly one deposit event processed | `count(deposit_events WHERE order_id = X) == 1` and `status == "PROCESSED"` |
| Exactly one worker applied | Among 20 results, exactly 1 has `already_processed=False`, 19 have `already_processed=True` |

### Implementation details

- **Thread pool:** `ThreadPoolExecutor(max_workers=20)`; each task runs `process_deposit_event(session, payload)` in its own session.
- **Transactions:** Each call to `process_deposit_event` uses a single DB transaction (BEGIN … COMMIT). The winner commits; the rest roll back on duplicate key and then read the cached row in a new transaction.
- **Concurrency safeguards:** Unique `idempotency_key` (claim-first) + row-level lock on the order (`get_for_update`) ensure that only one worker can apply the deposit.
