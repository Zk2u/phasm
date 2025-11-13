# PHASM Core Concepts

## What is PHASM?

PHASM (Phallible ASync State Machines) is a framework for building **deterministic, testable, and crash-recoverable** state machines with async operations and fallible state access.

## The Problem PHASM Solves

Traditional state machines assume:
- State is in-memory and instantly accessible
- Transitions are synchronous and infallible
- Side effects happen during transitions

In real systems:
- State might be in a database (fallible, async)
- External systems must be called (async, can fail)
- Crashes happen mid-transition

PHASM addresses these by:
1. Making fallibility and async first-class
2. Separating state mutations from external side effects
3. Enabling deterministic testing and crash recovery

## Core Architecture

```
Input → STF → (Updated State, Actions)
         ↓
    Actions executed externally
         ↓
    Results fed back as Input
```

### Components

#### 1. State
Your application state - can be:
- In-memory struct (HashMap, Vec, custom types)
- Database transaction (accessed via `state` parameter)
- Any storage accessed through the `state` parameter

**Rule**: Must be recoverable after crash (persisted or reconstructible from database).

**Important**: Mutations to state (including database writes through `state`) are NOT side effects - they're the core state transition. Only operations outside of `state` are side effects.

#### 2. Input
Two types:
- **Normal Input**: User requests, external events, timers
- **Tracked Action Results**: Results from previously emitted tracked actions

**Rule**: ALL external data must come through Input. This means:
- ✅ Reading/writing database through `state` parameter: **Allowed** (it's state mutation)
- ❌ Opening new database connections in STF: **Forbidden** (non-deterministic)
- ❌ Making HTTP calls to external services: **Forbidden** (use actions instead)
- ❌ Reading system time directly: **Forbidden** (pass as input)
- ❌ Reading from external APIs: **Forbidden** (pass results as input)

#### 3. STF (State Transition Function)
Pure function: `(State, Input) → (State', Actions)`

**Properties**:
- Deterministic: Same state + input = same output
- Atomic: Either succeeds completely or leaves state unchanged
- No external side effects: Only mutates state (including database writes via `state`) and emits action descriptions

#### 4. Actions
Descriptions of side effects to execute:

**Tracked Actions**: 
- Require confirmation/results
- Stored in state for crash recovery
- Examples: External API calls, payment processing, calling other services

**Untracked Actions**:
- Fire-and-forget
- Not recovered after crashes
- Examples: Notifications, logs, metrics, analytics

**Note**: Database writes through the `state` parameter are NOT actions - they're state mutations. Actions are for external operations outside of your state.

## The Key Insight

By separating **state mutations** (including database writes via `state`) from **external side effects** (actions), we get:

1. **Determinism**: STF is deterministic, testable function
2. **Crash Recovery**: Tracked actions stored in state
3. **Flexibility**: Execute actions however you want
4. **Testability**: Can simulate millions of transitions
5. **Clear Boundaries**: State mutations (including DB) vs. external calls are explicit

## Example: Payment Processing

```rust
struct PaymentSystem {
    pending_payments: HashMap<u64, Payment>,
    confirmed_payments: Vec<u64>,
    next_id: u64,
}

enum PaymentInput {
    ProcessPayment { amount: f32, user: String },
}

enum PaymentAction {
    ChargeCard { payment_id: u64, amount: f32 },
}

enum PaymentResult {
    Success { transaction_id: String },
    Failed { reason: String },
}

impl StateMachine for PaymentSystem {
    async fn stf(state: &mut State, input: Input, actions: &mut Actions) {
        match input {
            Input::Normal(ProcessPayment { amount, user }) => {
                // Generate deterministic ID from state
                let payment_id = state.next_id;
                state.next_id += 1;
                
                // Store in state BEFORE emitting action
                state.pending_payments.insert(payment_id, Payment {
                    amount,
                    user: user.clone(),
                    status: Pending,
                });
                
                // Emit tracked action to charge card
                actions.add(Action::Tracked(
                    TrackedAction::new(payment_id, ChargeCard { payment_id, amount })
                ))?;
                
                // Emit untracked notification
                actions.add(Action::Untracked(
                    NotifyUser { user, message: "Processing payment..." }
                ))?;
            }
            
            Input::TrackedActionCompleted { id: payment_id, res } => {
                let payment = state.pending_payments.get_mut(&payment_id)?;
                
                match res {
                    Success { transaction_id } => {
                        payment.status = Confirmed;
                        payment.transaction_id = Some(transaction_id);
                        state.confirmed_payments.push(payment_id);
                        
                        actions.add(Action::Untracked(
                            NotifyUser { user: payment.user, message: "Payment confirmed!" }
                        ))?;
                    }
                    Failed { reason } => {
                        payment.status = Failed;
                        payment.failure_reason = Some(reason);
                    }
                }
            }
        }
    }
    
    async fn restore(state: &State, actions: &mut Actions) {
        actions.clear()?;
        
        // Restore all pending payments
        for (payment_id, payment) in &state.pending_payments {
            if payment.status == Pending {
                // Re-check status with payment processor
                actions.add(Action::Tracked(
                    TrackedAction::new(*payment_id, CheckPaymentStatus { payment_id })
                ))?;
            }
        }
    }
}
```

## Crash Recovery Flow

1. System crashes with pending payment
2. On restart, load state from disk:
   ```rust
   {
       pending_payments: { 123: Payment { status: Pending, ... } },
       next_id: 124,
   }
   ```
3. Call `restore()`:
   - Sees payment 123 is pending
   - Emits `CheckPaymentStatus(123)` tracked action
4. Execute action, get result
5. Feed result back through `stf()` as `TrackedActionCompleted`
6. Payment marked confirmed or failed

## Why This Works

**Determinism**: If payment 123 is pending in state, restore ALWAYS emits the same action.

**Atomicity**: If STF fails before storing in `pending_payments`, the tracked action is never emitted.

**Testability**: Can simulate crash at any point and verify recovery works correctly.

## Next Steps

- [Critical Invariants](02_invariants.md) - Rules for correctness
- [Performance Guide](03_performance.md) - Optimizing state machines
- [Testing Guide](04_testing.md) - Simulation and property testing
