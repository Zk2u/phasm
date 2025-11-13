//! Phallible ASync State Machines (PHASM)
//!
//! A framework for building deterministic, testable, and crash-recoverable state machines
//! with async operations and fallible state access.
//!
//! # Core Concept
//!
//! PHASM separates state machine logic from external side effects:
//! - **State Transition Function (STF)**: Deterministic logic that mutates state (including
//!   database transactions accessed via the `state` parameter), validates inputs, and emits
//!   action descriptions for external operations
//! - **Actions**: Descriptions of external side effects (HTTP calls to other services,
//!   notifications, analytics) that are executed *after* STF completes successfully
//! - **State**: Can be in-memory structs, database transactions, or any storage accessed
//!   through the `state` parameter. Mutations to state are NOT side effects.
//! - **Restore**: Recovers pending operations from persisted state after crashes
//!
//! # Critical Invariants
//!
//! For a PHASM state machine to be theoretically sound:
//!
//! 1. **STF Atomicity**: If STF returns `Err`, state MUST be unchanged
//! 2. **Determinism**: Same state + same input = same output (always)
//! 3. **State Validity**: State must satisfy invariants at all times
//! 4. **No External Side Effects**: STF mutates state (including database writes via `state`)
//!    and emits action descriptions, but must not make HTTP calls or access external services
//! 5. **Tracked Actions in State**: Store pending tracked actions in state before emitting
//!
//! See module documentation in `docs/` for detailed rules and best practices.
//!
//! # Example
//!
//! ```ignore
//! struct MyStateMachine {
//!     counter: u64,
//!     pending_ops: HashMap<u64, PendingOp>,
//! }
//!
//! impl StateMachine for MyStateMachine {
//!     // Define your state machine...
//! }
//! ```

pub mod actions;

use crate::actions::{ActionsContainer, TrackedActionTypes};

/// Input to a state machine's STF.
///
/// # Variants
///
/// - [`Input::Normal`]: Regular input from users or external systems
/// - [`Input::TrackedActionCompleted`]: Result of a tracked action that was previously emitted
///
/// # Important
///
/// All external data (time, database reads, API responses) MUST be included in the input.
/// STF should never make external reads - it must be a pure function of state and input.
///
/// ```ignore
/// // ❌ WRONG
/// fn stf(state: &mut State, input: UserRequest) {
///     let now = SystemTime::now(); // Non-deterministic!
/// }
///
/// // ✅ CORRECT
/// fn stf(state: &mut State, input: Input<_, (UserRequest, SystemTime)>) {
///     let (request, timestamp) = match input {
///         Input::Normal((req, ts)) => (req, ts),
///         ...
///     };
/// }
/// ```
pub enum Input<TA: TrackedActionTypes, T> {
    Normal(T),
    TrackedActionCompleted { id: TA::Id, res: TA::Result },
}

/// A trait for describing a fallible, asynchronous state machine.
///
/// # Theory of Operation
///
/// A PHASM state machine is a deterministic function: `(State, Input) -> (State', Actions)`.
/// The STF reads current state and input, validates transitions, updates state atomically,
/// and emits actions describing side effects to perform.
///
/// # Critical Rules for Correctness
///
/// ## 1. STF Atomicity
///
/// If STF returns `Err`, state MUST remain unchanged:
///
/// ```ignore
/// // ✅ Validate before mutating
/// if !self.state.is_valid_transition(input) {
///     return Err(InvalidTransition); // State unchanged
/// }
/// self.state.apply(input); // Only mutate after validation
/// ```
///
/// ## 2. Determinism
///
/// No randomness, system time, or external reads in STF:
///
/// ```ignore
/// // ❌ Non-deterministic
/// let id = Uuid::new_v4();
/// let now = SystemTime::now();
///
/// // ✅ Deterministic - from state or input
/// let id = self.state.next_id;
/// self.state.next_id += 1;
/// ```
///
/// ## 3. State Always Valid
///
/// After every STF (success or failure), invariants must hold:
///
/// ```ignore
/// impl MyState {
///     fn check_invariants(&self) -> Result<(), String> {
///         // Verify no overlaps, consistency, etc.
///     }
/// }
/// ```
///
/// ## 4. Tracked Actions Must Be Stored in State
///
/// Before emitting a tracked action, store enough data in state to recreate it:
///
/// ```ignore
/// // Store in state first
/// self.state.pending.insert(req_id, request);
/// // Then emit action
/// actions.add(Action::Tracked(TrackedAction::new(req_id, ...)))?;
/// ```
///
/// ## 5. No Side Effects in STF
///
/// STF only mutates state and emits action *descriptions*:
///
/// ```ignore
/// // ❌ Side effect in STF
/// send_email(&email)?;
///
/// // ✅ Emit action for later execution
/// actions.add(Action::Untracked(SendEmail { to: email }))?;
/// ```
///
/// ## 6. Restore Must Be Pure Function of State
///
/// Restore can only read from state parameter:
///
/// ```ignore
/// fn restore(state: &State, actions: &mut Actions) {
///     // ✅ Restore from state
///     for (id, pending) in &state.pending {
///         if pending.needs_retry {
///             actions.add(Action::Tracked(...));
///         }
///     }
///     // ❌ Cannot query external systems
///     // let pending = database.query_pending().await; // NO!
/// }
/// ```
///
/// # Testing
///
/// PHASM enables deterministic simulation testing:
///
/// ```ignore
/// let mut rng = ChaCha8Rng::seed_from_u64(12345); // Deterministic!
/// for _ in 0..10000 {
///     let input = generate_random_input(&mut rng);
///     state_machine.stf(state, input, actions).await?;
///     state.check_invariants()?; // Verify after every transition
/// }
/// ```
///
/// Same seed = same test execution = reproducible bugs.
pub trait StateMachine {
    /// Type group for Tracked Action - actions that are retryable, restorable
    /// and whose result is given to the state machine after completion.
    type TrackedAction: TrackedActionTypes;
    /// Type for untracked actions - actions that are "fire and forget".
    type UntrackedAction;

    /// Type for a collection of which actions produced by a state transition
    /// can be placed.
    type Actions: ActionsContainer<Self::UntrackedAction, Self::TrackedAction>;

    /// State/data of the state machine.
    type State;
    /// Input type for a single STF invocation
    type Input;

    /// An error that can occur during STF
    type TransitionError;
    /// An error that can occur during state machine restoration
    type RestoreError;

    /// The future type for the State Transition Function.
    type StfFuture<'state, 'actions>: Future<Output = Result<(), Self::TransitionError>>;
    /// The future type for the State Machine Restoration.
    type RestoreFuture<'state, 'actions>: Future<Output = Result<(), Self::RestoreError>>;

    /// The core State Transition Function.
    ///
    /// # Semantics
    ///
    /// STF is a pure, deterministic, atomic function:
    /// - **Input**: Current state + input
    /// - **Output**: Updated state + actions to execute
    /// - **Atomicity**: If returns `Err`, **state** MUST be unchanged (but actions can be modified)
    /// - **Determinism**: Same state + input always produces same output
    ///
    /// # Parameters
    ///
    /// - `state`: Mutable reference to current state. Modify this to reflect the transition.
    /// - `input`: The input triggering this transition (user request or tracked action result)
    /// - `actions`: Container to emit actions into. DO NOT read from this - it's for output only.
    ///   The container is passed to reuse allocations across calls. **You can add actions even
    ///   before returning an error** - the caller clears it regardless of success/failure.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Transition successful, state updated, actions emitted
    /// - `Err(TransitionError)`: Transition failed, **state** MUST be unchanged (actions can be modified)
    ///
    /// # Critical Rules
    ///
    /// 1. **Validate before mutating state**: Check all preconditions before changing **state**.
    ///    However, you can emit actions (like error messages) before returning errors.
    /// 2. **Store tracked actions in state**: Before emitting a tracked action, store enough
    ///    data in state that `restore()` can recreate it
    /// 3. **No external reads**: All external data must come through `input`. Note: reading/writing
    ///    to a database through `state` is fine - it's external *connections* that are forbidden.
    /// 4. **No external side effects**: Only mutate state and emit action descriptions. Don't make
    ///    HTTP calls, don't write to external services. Database writes through `state` are fine.
    ///
    /// # Example
    ///
    /// ```ignore
    /// async fn stf(
    ///     state: &mut MyState,
    ///     input: Input<MyTracked, MyInput>,
    ///     actions: &mut Actions,
    /// ) -> Result<(), MyError> {
    ///     match input {
    ///         Input::Normal(user_request) => {
    ///             // 1. Validate BEFORE mutating state (but can emit actions)
    ///             if !state.can_process(&user_request) {
    ///                 // Optional: emit error feedback action
    ///                 actions.add(Action::Untracked(ShowError { msg: "Invalid" }))?;
    ///                 return Err(MyError::InvalidRequest);
    ///             }
    ///
    ///             // 2. Store in state for restore
    ///             let req_id = state.next_id;
    ///             state.next_id += 1;
    ///             state.pending.insert(req_id, user_request.clone());
    ///
    ///             // 3. Emit tracked action
    ///             actions.add(Action::Tracked(
    ///                 TrackedAction::new(req_id, ExternalCall { ... })
    ///             ))?;
    ///
    ///             // 4. Emit untracked actions
    ///             actions.add(Action::Untracked(
    ///                 SendNotification { user: user_request.user }
    ///             ))?;
    ///
    ///             Ok(())
    ///         }
    ///         Input::TrackedActionCompleted { id, res } => {
    ///             // Update state based on action result
    ///             let pending = state.pending.get_mut(&id)
    ///                 .ok_or(MyError::UnknownRequest)?;
    ///             pending.status = match res {
    ///                 Success => Status::Completed,
    ///                 Failed => Status::Failed,
    ///             };
    ///             Ok(())
    ///         }
    ///     }
    /// }
    /// ```
    fn stf<'state, 'actions>(
        state: &'state mut Self::State,
        input: Input<Self::TrackedAction, Self::Input>,
        actions: &'actions mut Self::Actions,
    ) -> Self::StfFuture<'state, 'actions>;

    /// Restore tracked actions from state after crash/restart.
    ///
    /// # Purpose
    ///
    /// After a system crash, `restore()` rebuilds the list of pending tracked actions
    /// that need to be retried or checked for completion.
    /// **Rule**: Restore can ONLY read from the `state` parameter.
    ///
    /// # Semantics
    ///
    /// - **Input**: Current state (after loading from disk/database or from transaction)
    /// - **Output**: Actions that should be re-executed or status-checked
    /// - **Purity**: Must be a pure function of `state` - no external reads beyond `state`
    ///
    /// # Parameters
    ///
    /// - `state`: The restored state (loaded from persistent storage)
    /// - `actions`: Container to emit restored actions into
    ///
    /// # Critical Rules
    ///
    /// 1. **Only use state**: Cannot open new database connections or query external APIs.
    ///    Reading from a database through `state` is fine - it's opening new connections that's forbidden.
    /// 2. **Must be deterministic**: Same state always produces same actions
    /// 3. **Clear before use**: The actions container should be cleared before adding
    ///
    /// # Example
    ///
    /// ```ignore
    /// async fn restore(
    ///     state: &MyState,
    ///     actions: &mut Actions,
    /// ) -> Result<(), RestoreError> {
    ///     // Clear container to reuse allocation
    ///     actions.clear()?;
    ///
    ///     // Restore all pending tracked actions from state
    ///     for (id, pending) in &state.pending_operations {
    ///         match pending.status {
    ///             Status::AwaitingResponse => {
    ///                 // Re-check status with external system
    ///                 actions.add(Action::Tracked(
    ///                     TrackedAction::new(id, CheckStatus { id })
    ///                 ))?;
    ///             }
    ///             Status::NeedsRetry => {
    ///                 // Retry the original operation
    ///                 actions.add(Action::Tracked(
    ///                     TrackedAction::new(id, pending.original_action.clone())
    ///                 ))?;
    ///             }
    ///             Status::Completed => {
    ///                 // Already done, skip
    ///             }
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Testing Restore
    ///
    /// Verify restore correctness:
    ///
    /// ```ignore
    /// // Create state with pending operations
    /// let mut state = MyState {
    ///     pending: hashmap! { 1 => PendingOp { ... } },
    ///     ...
    /// };
    ///
    /// // Restore should recreate the tracked actions
    /// let mut actions = vec![];
    /// MyStateMachine::restore(&state, &mut actions).await?;
    ///
    /// assert_eq!(actions.len(), 1);
    /// assert!(matches!(actions[0], Action::Tracked(_)));
    /// ```
    fn restore<'state, 'actions>(
        state: &'state Self::State,
        actions: &'actions mut Self::Actions,
    ) -> Self::RestoreFuture<'state, 'actions>;
}
