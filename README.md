# PHASM - Phallible Async State Machines

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

A Rust framework for building **deterministic, testable, and crash-recoverable** state machines with async operations and fallible state access.

**Build systems that are correct by construction** - payments, bookings, workflows, and distributed systems with guarantees that go beyond traditional testing.

## Why PHASM?

Traditional state machines break down in production - race conditions, crashes mid-operation, and bugs that only appear under load. PHASM solves this by making correctness **verifiable**:

- 🎯 **Deterministic execution** - Same inputs always produce same outputs (reproducible bugs!)
- 🔄 **Crash recovery** - Resume from any failure point automatically
- 🧪 **Simulation testing** - Verify correctness across millions of random operations in seconds
- 🔒 **Race condition handling** - Deterministic conflict resolution built-in
- 💾 **Flexible state** - In-memory, database transactions, or hybrid

**Real results**: The dentist booking example verifies 90,000+ operations (including race conditions, crashes, and payment failures) in ~4 seconds - finding bugs humans would never think to test.

## Why PHASM Was Created

**The Theory-Practice Gap**

Traditional state machine theory is elegant and provably correct - but assumes synchronous, infallible operations and in-memory state. Real systems need:
- Async operations (network calls, database transactions)
- Fallible operations (APIs fail, databases timeout)
- Persistence (crashes happen, state must survive)
- Scale (millions of operations, distributed systems)

Existing frameworks force a choice: theoretical correctness OR practical engineering.

**PHASM bridges this gap.**

PHASM expands the theoretical state machine model to allow for **theoretical correctness while interoperating with real-world engineering**:

- **Async-first**: State transitions can await - database transactions, validation checks
- **Fallible state access**: Database connections can fail, STF can return errors atomically
- **Separation of state and effects**: State mutations (including DB writes) remain deterministic; external effects are explicit
- **Tracked actions**: Theoretical model extended with action results feeding back as inputs
- **Crash recovery**: Restore function makes the model crash-safe without losing correctness

The result: Build high-performance, scalable systems with the same correctness guarantees as theoretical state machines, but with the flexibility to handle real-world requirements like databases, external APIs, and failures.

**PHASM is not a compromise - it's an expansion.** You get both theoretical soundness AND practical engineering.

## How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                     PHASM Architecture                       │
└─────────────────────────────────────────────────────────────┘

  Input (user request, time, external data)
    │
    ▼
┌─────────────────────────────────────────┐
│  State Transition Function (STF)        │
│  ────────────────────────────────       │
│  • Validates inputs                     │
│  • Mutates state (incl. database)       │
│  • Emits action descriptions            │
│  • Atomic: error = no changes           │
└─────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────┐
│  Updated State + Actions                 │
│  ────────────────────────────           │
│  State persisted (in-memory or DB)      │
│  Actions executed externally            │
└─────────────────────────────────────────┘
    │
    ├──► Tracked Actions (payment, API calls)
    │    │
    │    ▼
    │    External Systems
    │    │
    │    └──► Results feed back as Input ──┐
    │                                       │
    └──► Untracked Actions (notifications)  │
                                            │
                                            ▼
                                         (loop)

After Crash: restore(state) → re-emit pending actions
```

## Quick Example

```rust
use phasm::{Input, StateMachine};

struct PaymentSystem {
    balance: u64,
    pending: HashMap<u64, Payment>,
    next_id: u64,
}

impl StateMachine for PaymentSystem {
    async fn stf(
        state: &mut Self::State,
        input: Input<Self::TrackedAction, Self::Input>,
        actions: &mut Self::Actions,
    ) -> Result<(), Self::TransitionError> {
        match input {
            Input::Normal(ProcessPayment { amount, user }) => {
                // Validate before mutating
                if state.balance < amount {
                    return Err(InsufficientFunds);
                }

                // Generate deterministic ID
                let payment_id = state.next_id;
                state.next_id += 1;

                // Store in state before emitting action
                state.pending.insert(payment_id, Payment { amount, user });

                // Emit tracked action (will be retried on crash)
                actions.add(Action::Tracked(
                    TrackedAction::new(payment_id, ChargeCard { amount })
                ))?;

                Ok(())
            }
            Input::TrackedActionCompleted { id, res } => {
                // Handle payment result
                let payment = state.pending.get_mut(&id)?;
                payment.status = if res.is_success() {
                    state.balance -= payment.amount;
                    Confirmed
                } else {
                    Failed
                };
                Ok(())
            }
        }
    }

    async fn restore(state: &Self::State, actions: &mut Self::Actions) -> Result<(), Self::RestoreError> {
        // Recreate pending actions from state after crash
        for (id, payment) in &state.pending {
            if payment.status == Pending {
                actions.add(Action::Tracked(
                    TrackedAction::new(*id, CheckPaymentStatus { id })
                ))?;
            }
        }
        Ok(())
    }
}
```

## Core Concepts

### State Transition Function (STF)
Deterministic function: `(State, Input) → (State', Actions)`

- Validates inputs and mutates state (including database writes via `state`)
- Emits action descriptions (not executions)
- Must be atomic: error = no state changes

### Actions
Descriptions of external operations executed after STF succeeds:

- **Tracked**: Perfect for long-running background operations that produce results and can fail
  - Examples: Payment processing, external API calls, background jobs
  - Results feed back as `Input::TrackedActionCompleted`
  - Stored in state for crash recovery and retry
  - Use when operation outcome affects system correctness

- **Untracked**: Fire-and-forget operations whose execution doesn't affect correctness
  - Examples: Logs, metrics, analytics, notifications, UI updates
  - Not recovered after crashes
  - Use when you need to emit information but don't need confirmation

### Restore
Recovers pending operations from state after crashes by reading state and re-emitting actions.

## Key Requirements

### ✅ What You Must Do

1. **Validate before mutating** - Check all conditions before changing state
2. **Use deterministic IDs** - Generate from state counters, not `SystemTime::now()` or random
3. **Store tracked actions in state** - Before emitting, so restore can recreate them
4. **Return all external data via Input** - Time, API responses, database reads (if not via `state`)
5. **State atomicity** - If STF returns `Err`, state must be unchanged

### ❌ What You Must Not Do

1. **No external side effects in STF** - No HTTP calls, no opening new connections
2. **No randomness** - No `rand::random()`, no unseeded RNGs
3. **No system time** - No `SystemTime::now()`, pass time via Input
4. **No external reads** - No database connections (unless via `state` parameter)

### ✨ What's Allowed (Not Side Effects!)

- ✅ Writing to in-memory data structures
- ✅ Writing to database **through `state` parameter** (e.g., `state.txn.set()`)
- ✅ Modifying actions container before returning errors (caller clears it)

## State Can Be Anything

```rust
// In-memory
struct State {
    users: HashMap<u64, User>,
}

// Database transaction
struct State<'txn> {
    txn: &'txn mut Transaction,
}

// Both follow the same rules!
```

## Testing

The killer feature - deterministic simulation:

```rust
use rand_chacha::ChaCha8Rng;

#[test]
async fn test_correctness() {
    let mut rng = ChaCha8Rng::seed_from_u64(12345); // Deterministic!
    let mut state = MySystem::new();

    for i in 0..100_000 {
        let input = generate_random_input(&mut rng);
        MySystem::stf(&mut state, input, &mut actions).await.ok();

        // Check invariants after EVERY operation
        state.check_invariants()
            .expect(&format!("Invariant violated at {}", i));
    }
}
```

Same seed = same test execution = reproducible bugs.

## When to Use PHASM

### ✅ Great For
- Payment processing
- Reservation systems (hotels, appointments, flights)
- Workflow engines (approvals, multi-step processes)
- E-commerce (inventory, orders)
- Distributed systems requiring correctness

### ❌ Overkill For
- Simple CRUD apps
- Stateless services
- Read-only systems
- Prototypes (unless correctness is critical)

## Examples

- **`examples/coffee_shop.rs`** - Loyalty points redemption with tracked actions
- **`examples/csm.rs`** - Simple counter state machine
- **`dentist_booking/`** - Full appointment booking system with comprehensive tests
  - 5 integration tests + 8 simulation tests
  - 90,000+ operations tested in ~4 seconds
  - Verifies all bookings match user preferences

Run examples:
```bash
cargo run --example coffee_shop
cd dentist_booking && cargo test
```

## Documentation

- **[Docstrings in `src/lib.rs`](src/lib.rs)** - Detailed API documentation
- **[Core Concepts](docs/01_core_concepts.md)** - Architecture and examples
- **[Critical Invariants](docs/02_invariants.md)** - Rules for correctness
- **[Performance Guide](docs/03_performance.md)** - Optimization strategies
- **[Testing Guide](docs/04_testing.md)** - Simulation testing patterns
- **[Database State](docs/05_database_state.md)** - Using databases as state

## Quick Start

Add to `Cargo.toml`:
```toml
[dependencies]
phasm = "0.2"
```

## Where to Start

**New to PHASM?** Follow this path:

1. **Understand the basics** (5 min)
   - Read the [Quick Example](#quick-example) above
   - Skim [Core Concepts](#core-concepts) to understand STF, Actions, and Restore

2. **See it in action** (10 min)
   ```bash
   cargo run --example coffee_shop
   ```
   - Shows tracked actions for point redemption
   - Demonstrates error handling and state atomicity
   - Includes crash recovery simulation

3. **Learn the rules** (15 min)
   - Read [Key Requirements](#key-requirements) section above
   - These are the critical invariants you must follow
   - Understand what's allowed vs forbidden

4. **Study a complete example** (30 min)
   ```bash
   cd dentist_booking
   cargo test -- --nocapture
   ```
   - Production-ready appointment booking system
   - See how preferences are validated
   - Observe 90,000+ operations tested in seconds
   - Read [dentist_booking/README.md](dentist_booking/README.md)

5. **Deep dive** (1-2 hours)
   - [Critical Invariants](docs/02_invariants.md) - Detailed rules with examples
   - [Testing Guide](docs/04_testing.md) - Simulation testing patterns
   - [Database State](docs/05_database_state.md) - If you need database-backed state

6. **Build your state machine**
   - Copy the pattern from `dentist_booking/src/lib.rs`
   - Define your State, Input, Actions
   - Implement STF with validation-first approach
   - Write simulation tests to verify correctness

**Quick Reference**: The [docstrings in src/lib.rs](src/lib.rs) contain detailed API documentation with inline examples.

## Performance

Phasm doesn't affect performance of systems. You can use actions to offload
compute or split work across multiple state transitions. You can build correct,
testable and performant systems using phasm.

## License

MIT OR Apache-2.0

## Contributing

See the examples and documentation. When adding features, include:
- Simulation tests demonstrating correctness
- Documentation explaining the "why" not just the "what"
- Examples showing both correct and incorrect usage
