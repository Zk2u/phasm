//! Phallible ASync State Machines
//!
//! A way of building state machines in a practical way, focusing on an
//! asynchronous and fallible State Transition Function and restore operation.
//!
//! It supports [`Action`]s for creating side effects from a State Transition.

pub mod actions;

use crate::actions::{ActionsContainer, TrackedActionTypes};

pub enum Input<TA: TrackedActionTypes, T> {
    Normal(T),
    TrackedActionCompleted { id: TA::Id, res: TA::Result },
}

/// A trait for describing a fallible,
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
    type RestoreFuture<'state, 'actions>: Future<Output = Result<Self::Actions, Self::RestoreError>>;

    /// The core State Transition Function.
    ///
    /// Accepts previous state, input and an actions container to place actions in.
    ///
    /// The actions container should NOT be read from and is only passed through
    /// so an existing allocation can be reused.
    ///
    /// Tracked Actions MUST be stored in `state` before being returned in `actions` so they are restorable.
    fn stf<'state, 'actions>(
        state: &'state mut Self::State,
        input: Input<Self::TrackedAction, Self::Input>,
        actions: &'actions mut Self::Actions,
    ) -> Self::StfFuture<'state, 'actions>;

    /// Restore Tracked Actions from state on initialization
    fn restore<'state, 'actions>(
        state: &'state Self::State,
        actions: &'actions mut Self::Actions,
    ) -> Self::RestoreFuture<'state, 'actions>;
}
