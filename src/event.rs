use std::time::Duration;

pub enum Event {
    Quit,
    ToggleTimer,
    ResetTimer,
    SkipActivity,
    ExtendActivity(Duration),
    ReduceActivity(Duration),
}

pub struct EventConversionUndefinedError;
