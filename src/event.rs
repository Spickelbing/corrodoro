use std::time::Duration;

/// These are the events that can be sent to the app, either from a UI or via network.
pub enum Event {
    Quit,
    ToggleTimer,
    ResetTimer,
    SkipActivity,
    ExtendActivity(Duration),
    ReduceActivity(Duration),
}
