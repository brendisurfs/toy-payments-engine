# Toy Payments Engine 

by Brendan Prednis

---

## Overview 

This is a streaming payments engine implementation that processes deposits, withdrawals, disputes, resolutions, and chargebacks from a CSV file input,
outputting final account states to stdout.

## How to Run

To run the project: 
```
cargo run -- transactions.csv > accounts.csv
```

Tracing logs go to stderr deliberately, so it doesn't pollute the CSV output on stdout.

## Architecture and Structure

### `parser`

Parser handles CSV ingestion, as well as `StringRecord` deserialization into `RawRow`, which is then pattern-matched and parsed into 
typed `PaymentRecord` enum variants.

### `transactions`

The  `transactions` module houses all domain types related to incoming transactions from the CSV file,
such as `Transaction`, `PaymentEvent`, and `TransactionStatus`.
This module also owns the `on_next_transaction` dispatch function, which does the main routing for incoming `Transaction`s.

### `accounts`

This module owns the `AccountManager`, the main entity that facilitates all fund mutation logic, as well as the transaction log.

### `cli`

The `cli` module holds the logic for argument parsing, kept isolated so that the input source could be swapped in the future.


## Design Decisions 

### Streaming over Loading

Rows are processed one at a time via an iterator. 
The dataset is never fully loaded into memory, which allows for arbitrarily large inputs. 

### In-Memory Transaction Log

The `HashMap<u32, Transaction>` log in `AccountManager` is an architectural limitation, as it grows with every deposit. 
I deliberately chose to store only `Deposit` transactions as these are the only transactions subject to dispute.
Withdrawals represent funds already debited from the account, and reversing them would involve different settlement mechanics outside the scope of this implementation. 
I initially started with storing transactions inside the transaction_log as `Rc<RefCell<Transaction>>`, but `Transaction` is small in size and the added complexity did not align with the overall goals of this project.
The current implementation is acceptable for a toy project like this. In production, however, an append-only database would be more appropriate, offering durability and crash recovery. 

### Deposit-Only Disputes

The current implementation only operates on `Deposit` transactions.
This is a deliberate assumption, as `Withdrawal` transactions represent funds already leaving the system. 
Reversing them would involve different real-world mechanisms compared to reversing a deposit. 

### Decimal Precision 

All outputs are rounded to 4 decimal places to ensure consistent output, regardless of intermediate arithmetic.

### Type-Driven Correctness 

Using `TryFrom` for `Transaction`/`PaymentEvent` from `RawRow` eliminates invalid records.
By the time a record reaches the account manager, it is structurally valid. 

## Testing

### Manual CSV Inputs 

I used AI to build a testing CSV dataset that would test edge cases like duplicate transaction IDs, mismatched client IDs, and operations on a frozen account.
By using a deliberate dataset covering every edge case, I could manually test each safeguard within the transaction handler. 

### `test_account_deposit`

This tests creating an account via a basic deposit transaction and updates funds.

### `test_account_withdraw`

Tests that withdraw transactions on accounts with insufficient funds are rejected, and that valid withdrawals succeed. 

### `test_dispute_then_chargeback`

This tests the full cycle of dispute to chargeback:
```
deposit -> dispute -> chargeback
```
It verifies that we get proper available/held/total fund values at each meaningful step, and that the account is frozen.

### `test_resolve`

We test the full cycle of dispute to resolve. 
It verifies that funds are fully restored from held and are available. 


## Error Handling 

Malformed data is logged to stderr and skipped. 
Invalid state transitions, such as resolving a non-disputed transaction, are logged as warnings and ignored. 

## Concurrency Considerations

The current implementation is intentionally single-threaded. 
The `AccountManager` owns all state with no shared memory primitives needed.

A naive concurrent extension would wrap shared state in `Arc<Mutex<AccountManager>>`, creating one lock for all state. 
This works, but would create a bottleneck as concurrent TCP streams serialize on that lock, eliminating the entire benefit of concurrency under high load. 

A more production-oriented approach would be to eliminate the in-memory transaction log entirely, in favor of an external append-only store, such as Postgres with a transactions table.
With this implementation, per-stream state becomes either stateless or sharded by client ID, so streams for different clients never contend.
The parsing and dispatch logic in `parser` and `on_next_transaction` is already stream agnostic. The input source is abstracted behind a `Read` trait,
so plugging in a TCP stream in place of a file is a trivial change. 

## AI Use 

As of the year 2026, AI has become a prevalent tool used by many software engineers. I am no exception, and for the safety of future software, I believe it is important to be fully transparent and explicit about AI use.
However, I used AI a bit differently than others may use for a project like this. 

I used AI as a code reviewer across multiple iterations, identifying hidden control flow bugs, flagging antipatterns, extended transaction data for testing, and helping explain *why* something was wrong.
I refused to let AI write implementation code for me.
I did not use AI until I had a foundation architecture laid out and I ran through ideas for solving this challenge.

### What AI caught

AI did help in catching unsound tests, such as the `test_resolve` test. What I was testing for was slightly off, and that was pointed out to me, but without giving me the correct fix. 
I still had to reason about fixing it myself. 
Another aspect was a non-enforced client_id guard, checking that the referenced transaction client id was the same as the reference client id being passed in.
A small but crucial check that adds a robust layer of protection to each mutation event. 

### AI Discussions

Many design decisions were discussed at length: the use of `Rc<RefCell<T>>` for transaction_log, concurrency reasoning, and discussing a coherent order for this README. 
I chose collaborative reasoning where I made the calls, and enabled myself to push back against AI with real arguments, choosing to make decisions against the reviewer's suggestion when I could justify my position confidently. 

### AI as a learning tool, not a servant 

For a project like this, using AI as a servant to write code or scaffold out the project to it's understanding of the spec defeats the entire purpose of this learning exercise. 
By only bringing in AI when I truly got stuck or wanted to rubber duck a decision I was making, I improved my critical thinking skills and verbal communication skills by using dictation rather than typing.
It allowed me to build confidence in my design decisions, or at least explain them in detail, rather than being unable to reason about my own work. 
I think this is a meaningful, productive use of AI, rather than having it just write code. 
In scenarios where you fully understand the spec, the scope is small, or have to scaffold out data types, it can be useful and time saving to have AI write code for you.
However, this is not that scenario, and I would be doing myself a disservice by deferring my thinking to AI. 

## Final Thoughts

In the future, I would like to expand this project out to more complex, real-world application. 

### Server Wrapping

Currently, `build_csv_reader` already accepts a generic `R: Read` trait, so the ingestion layer is ready for an incoming stream of data. 
The additional work would be around proper session management, backpressure for incoming transaction messages, and connection lifecycle.

### Profiling

`tracing` is already instrumented with `FmtSpan::CLOSE` timing, so the initial foundation for latency analysis is already there. 
What I would add in the future is more robust benchmarking or a flame graph, to help identify problem areas, especially if expanding the project to live in a production server.

### Transaction Log Separation

To improve transaction log handling, I would abstract `transaction_log` in `AccountManager` to its own struct with a defined trait interface, making it mockable in tests and swappable for a database implementation without touching `AccountManager`.

### Improved comments for internal processes 

Public facing APIs are documented, but internal logic comments could definitely be more explicit about why certain state transitions are guarded, not just what they do. 


