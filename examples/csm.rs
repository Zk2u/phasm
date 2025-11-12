use std::{
    future,
    pin::Pin,
    task::{Context, Poll},
};

use phasm::{
    Input, StateMachine,
    actions::{Action, ActionsContainer, TrackedActionTypes},
};

#[monoio::main]
async fn main() {
    let mut csm = CounterStateMachine { counter: 0 };
    let mut actions = Vec::new();
    CounterStateMachine::stf(&mut csm, Input::Normal(()), &mut actions)
        .await
        .unwrap();
    assert_eq!(
        actions,
        vec![Action::Untracked(CsmAction::Incremented { from: 0, to: 1 })]
    );
    for action in actions.iter() {
        match action {
            Action::Tracked(_) => unreachable!(),
            Action::Untracked(act) => match act {
                CsmAction::Incremented { from, to } => {
                    println!("Incremented from {} to {}", from, to);
                }
            },
        }
    }
    actions.clear();
}

struct CounterStateMachine {
    counter: u64, 
}

#[derive(Debug)]
enum CsmStfError {
    Overflowed,
    FailedToQueueAction,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CsmAction {
    Incremented { from: u64, to: u64 },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CsmTrackedAction;

impl TrackedActionTypes for CsmTrackedAction {
    type Id = ();
    type Action = ();
    type Result = ();
}

impl StateMachine for CounterStateMachine {
    type UntrackedAction = CsmAction;
    type TrackedAction = CsmTrackedAction;
    type Actions = Vec<Action<Self::UntrackedAction, Self::TrackedAction>>;

    type State = Self;
    type Input = ();

    type TransitionError = CsmStfError;
    type RestoreError = ();

    type StfFuture<'state, 'actions> = CsmStfFuture<'state, 'actions>;
    type RestoreFuture<'state, 'actions> = future::Ready<Result<Self::Actions, Self::RestoreError>>;

    fn stf<'state, 'actions>(
        state: &'state mut Self::State,
        _input: Input<Self::TrackedAction, Self::Input>,
        actions: &'actions mut Self::Actions,
    ) -> Self::StfFuture<'state, 'actions> {
        CsmStfFuture { state, actions }
    }

    fn restore<'state, 'actions>(
        _state: &'state Self::State,
        _actions: &'actions mut Self::Actions,
    ) -> Self::RestoreFuture<'state, 'actions> {
        future::ready(Ok(vec![]))
    }
}

struct CsmStfFuture<'state, 'actions> {
    state: &'state mut CounterStateMachine,
    actions: &'actions mut <CounterStateMachine as StateMachine>::Actions,
}

impl<'state, 'actions> Future for CsmStfFuture<'state, 'actions> {
    type Output = Result<(), <CounterStateMachine as StateMachine>::TransitionError>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = (|| {
            let prev = self.state.counter;
            let new = self
                .state
                .counter
                .checked_add(1)
                .ok_or(CsmStfError::Overflowed)?;
            self.state.counter = new;
            self.actions
                .add(Action::Untracked(CsmAction::Incremented {
                    from: prev,
                    to: new,
                }))
                .map_err(|_| CsmStfError::FailedToQueueAction)?;
            Ok(())
        })();
        Poll::Ready(result)
    }
}
