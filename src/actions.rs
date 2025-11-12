use std::fmt::Debug;

pub trait TrackedActionTypes {
    /// A type used to identify a tracked action within a given state machine.
    type Id: Debug + PartialEq + Eq + PartialOrd;
    /// A type used to represent the action to be performed.
    type Action: Debug + PartialEq + Eq;
    /// A type used to represent the result of the action.
    type Result: Debug;
}

#[derive(Debug, PartialEq, Eq)]
pub struct TrackedAction<Types: TrackedActionTypes> {
    action_id: Types::Id,
    action: Types::Action,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action<UA, TATypes: TrackedActionTypes> {
    Tracked(TrackedAction<TATypes>),
    Untracked(UA),
}

/// A trait for describing a fallible container for a set of [`Action`]s.
pub trait ActionsContainer<UA, TA: TrackedActionTypes> {
    type Error;
    /// Creates a new instance of the container. May fail if the container cannot be initialized.
    fn new() -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Creates a new instance of the container with a capacity hint. May fail if the container cannot be initialized.
    fn with_capacity(capacity: usize) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Clears the container. May fail if the container cannot be cleared.
    fn clear(&mut self) -> Result<(), Self::Error>;

    /// Adds an action to the container. May fail if the container cannot be modified.
    fn add(&mut self, action: Action<UA, TA>) -> Result<(), Self::Error>;
}

impl<UA, TA: TrackedActionTypes> ActionsContainer<UA, TA> for Vec<Action<UA, TA>> {
    type Error = ();

    fn new() -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Vec::new())
    }

    fn with_capacity(capacity: usize) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Vec::with_capacity(capacity))
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        self.clear();
        Ok(())
    }

    fn add(&mut self, action: Action<UA, TA>) -> Result<(), Self::Error> {
        self.push(action);
        Ok(())
    }
}
