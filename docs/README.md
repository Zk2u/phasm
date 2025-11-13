# PHASM Documentation

Comprehensive guides for building correct, performant, and testable state machines with PHASM.

## Getting Started

1. [Core Concepts](01_core_concepts.md) - Understanding PHASM's architecture and design
2. [Critical Invariants](02_invariants.md) - Rules for building sound state machines
3. [Performance Guide](03_performance.md) - Optimizing your state machines
4. [Testing Guide](04_testing.md) - Deterministic simulation testing
5. [Database-Backed State](05_database_state.md) - Using transactional databases as state

## Quick Links

### Core Concepts
- What is PHASM and why use it?
- State, Input, STF, Actions, and Restore
- Example: Payment processing system

### Critical Invariants
- STF Atomicity
- Determinism requirements
- State validity
- Tracked action storage
- Restore purity

### Performance
- Data structure selection (ahash, BTreeMap)
- Invariant checking strategies
- Memory efficiency
- In-memory optimization patterns
- Real-world benchmarks (1.4M ops/sec)

### Database-Backed State
- State as transactional database (FoundationDB, PostgreSQL)
- STF atomicity via database transactions
- Deterministic IDs from database sequences
- Restore from database state
- Hybrid in-memory + database patterns

### Testing
- Deterministic simulation with seeded RNGs
- Time-bounded test runners
- Property-based testing
- Crash recovery testing
- Race condition testing

## Philosophy

PHASM is designed around these principles:

1. **Determinism First**: Same state + input = same output (always)
2. **Explicit Over Implicit**: All state mutations are visible
3. **Separation of Concerns**: State mutations (including database writes via `state`) vs. external side effects (actions for HTTP calls, notifications, etc.)
4. **Crash Recovery**: System can always resume from persisted state
5. **Testability**: Simulation testing finds bugs humans miss
6. **Flexibility**: State can be in-memory, database transaction, or hybrid

## Common Patterns

### State Machine Skeleton

```rust
struct MyStateMachine {
    // Your state
    data: HashMap<Id, Data>,
    pending: HashMap<RequestId, PendingRequest>,
    next_id: u64,
}

impl StateMachine for MyStateMachine {
    type TrackedAction = MyTracked;
    type UntrackedAction = MyUntracked;
    type Actions = Vec<Action<...>>;
    type State = Self;
    type Input = MyInput;
    type TransitionError = MyError;
    type RestoreError = ();
    
    type StfFuture<...> = MyStfFuture<...>;
    type RestoreFuture<...> = future::Ready<Result<(), ()>>;
    
    fn stf(...) -> Self::StfFuture<...> {
        MyStfFuture { state, actions, input }
    }
    
    fn restore(...) -> Self::RestoreFuture<...> {
        actions.clear()?;
        for (id, pending) in &state.pending {
            // Recreate tracked actions from state
        }
        future::ready(Ok(()))
    }
}
```

### Invariant Checking

```rust
impl MyState {
    pub fn check_invariants(&self) -> Result<(), String> {
        // 1. Check consistency
        // 2. Check no conflicts
        // 3. Check referential integrity
        Ok(())
    }
}
```

### Simulation Testing

```rust
#[test]
async fn test_simulation() {
    let mut rng = ChaCha8Rng::seed_from_u64(12345);
    let mut state = MyStateMachine::new();
    
    for i in 0..100_000 {
        let input = generate_random_input(&mut rng);
        state.stf(input, &mut actions).await.ok();
        state.check_invariants()
            .expect(&format!("Invariant violated at {}", i));
    }
}
```

## Examples

See `dentist_booking/` crate for a full example:
- Weekly schedules with multiple time ranges
- Variable appointment durations
- Auto-selection algorithm
- Payment preauthorization
- Comprehensive simulation test suite
- 4.27M operations tested with full invariant checking

## Contributing

When adding new documentation:
- Use concrete code examples
- Explain the "why" not just the "what"
- Include both correct and incorrect patterns
- Link to relevant examples

## See Also

- [Main crate docs](../src/lib.rs) - Trait definitions with extensive docs
- [Actions module](../src/actions.rs) - Action system documentation
- [Dentist booking example](../dentist_booking/) - Full working example
