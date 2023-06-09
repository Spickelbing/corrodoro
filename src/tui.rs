use crate::app::NetworkStatus;
use crate::notification;
use crate::protocol::{Event, TimerVisuals};
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind,
};
use futures::StreamExt;
use std::io;
use std::time::Duration;
use thiserror::Error;
use tui::{backend::CrosstermBackend, Terminal};

mod animation;
mod render;
mod widgets;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    raw_mode_enabled: bool,
    alternate_screen_enabled: bool,
    event_stream: EventStream,
    show_settings: bool,
    show_timer: bool,
    last_display_data: Option<DisplayData>,
}

struct DisplayData {
    timer_visuals: TimerVisuals,
    network_status: NetworkStatus,
}

impl Tui {
    pub fn new() -> Result<Self, TuiError> {
        let backend = CrosstermBackend::new(io::stdout());

        Ok(Tui {
            terminal: Terminal::new(backend).map_err(TuiError::Creation)?,
            alternate_screen_enabled: false,
            raw_mode_enabled: false,
            event_stream: EventStream::new(),
            show_settings: true,
            show_timer: true,
            last_display_data: None,
        })
    }

    /// Has to be explicitly disabled, because disabling can cause errors that have to be catched.
    /// Is not disabled by dropping.
    pub fn enable(&mut self) -> Result<(), TuiError> {
        crossterm::terminal::enable_raw_mode().map_err(TuiError::RawModeToggle)?;
        self.raw_mode_enabled = true;

        crossterm::execute!(
            self.terminal.backend_mut(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        )
        .map_err(TuiError::AlternateScreenToggle)?;
        self.alternate_screen_enabled = true;

        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), TuiError> {
        if self.alternate_screen_enabled {
            crossterm::execute!(
                self.terminal.backend_mut(),
                crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::event::DisableMouseCapture,
            )
            .map_err(TuiError::AlternateScreenToggle)?;
        }
        if self.raw_mode_enabled {
            crossterm::terminal::disable_raw_mode().map_err(TuiError::RawModeToggle)?;
        }

        Ok(())
    }

    pub fn render(
        &mut self,
        timer_visuals: &TimerVisuals,
        network_status: &NetworkStatus,
    ) -> Result<(), TuiError> {
        self.terminal
            .draw(|f| {
                render::render_ui(
                    f,
                    timer_visuals,
                    network_status,
                    self.show_settings,
                    self.show_timer,
                );
            })
            .map_err(TuiError::Rendering)?;

        self.last_display_data = Some(DisplayData {
            timer_visuals: *timer_visuals,
            network_status: network_status.clone(),
        });

        Ok(())
    }

    pub fn show_notification(&self, msg: &str, audio: bool) {
        // ignore errors for now, perhaps add a log message in the tui in the future
        let _ = notification::show_desktop_notification("", msg);
        if audio {
            notification::play_notification_sound();
        }
    }

    pub async fn read_event(&mut self) -> Result<Event, TuiError> {
        loop {
            let crossterm_event = self.read_crossterm_event().await?;

            // just slapped in real quick, could do this nicer
            self.handle_crossterm_event(&crossterm_event)?;

            if let Ok(event) = Event::try_from(crossterm_event) {
                return Ok(event);
            }
        }
    }

    async fn read_crossterm_event(&mut self) -> Result<CrosstermEvent, TuiError> {
        let event = self.event_stream.next().await;
        let event = event
            .ok_or(TuiError::EventStreamClosed)?
            .map_err(TuiError::ReadInputEvent)?;

        Ok(event)
    }

    fn handle_crossterm_event(&mut self, event: &CrosstermEvent) -> Result<(), TuiError> {
        match event {
            CrosstermEvent::Key(key_event) if key_event.kind != KeyEventKind::Release => {
                match key_event.code {
                    KeyCode::Char('1') => self.toggle_settings(),
                    KeyCode::Char('2') => self.toggle_timer(),
                    _ => {}
                }
            }
            CrosstermEvent::Resize(_, _) => {
                if let Some(display_data) = self.last_display_data.take() {
                    self.render(&display_data.timer_visuals, &display_data.network_status)?;
                    self.last_display_data = Some(display_data);
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
    }

    fn toggle_timer(&mut self) {
        self.show_timer = !self.show_timer;
    }
}

#[derive(Debug, Error)]
pub enum TuiError {
    #[error("failed to initialize terminal ui: {0}")]
    Creation(io::Error),
    #[error("failed to toggle terminal raw mode: {0}")]
    RawModeToggle(io::Error),
    #[error("failed to toggle alternate terminal screen: {0}")]
    AlternateScreenToggle(io::Error),
    #[error("failed to render terminal ui: {0}")]
    Rendering(io::Error),
    #[error("failed to read input event from terminal: {0}")]
    ReadInputEvent(io::Error),
    #[error("terminal input event stream was closed unexpectedly")]
    EventStreamClosed,
}

pub struct EventConversionUndefinedError;

impl TryFrom<CrosstermEvent> for Event {
    type Error = EventConversionUndefinedError;

    // TODO: accept uppercase chars too
    fn try_from(event: CrosstermEvent) -> Result<Self, Self::Error> {
        match event {
            CrosstermEvent::Key(key_event)
                if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                match key_event.code {
                    KeyCode::Char('c') => Some(Event::Quit),
                    _ => None,
                }
            }
            CrosstermEvent::Key(key_event) if key_event.kind != KeyEventKind::Release => {
                match key_event.code {
                    KeyCode::Char('q') => Some(Event::Quit),
                    KeyCode::Char('r') => Some(Event::ResetTimer),
                    KeyCode::Char('s') => Some(Event::SkipActivity),
                    KeyCode::Char(' ') => Some(Event::ToggleTimer),
                    KeyCode::Up => Some(Event::ExtendActivity(Duration::from_secs(60))),
                    KeyCode::Down => Some(Event::ReduceActivity(Duration::from_secs(60))),
                    KeyCode::Esc => Some(Event::Quit),
                    _ => None,
                }
            }
            CrosstermEvent::Mouse(mouse_event) => match mouse_event.kind {
                MouseEventKind::ScrollUp => Some(Event::ExtendActivity(Duration::from_secs(60))),
                MouseEventKind::ScrollDown => Some(Event::ReduceActivity(Duration::from_secs(60))),
                _ => None,
            },
            _ => None,
        }
        .ok_or(EventConversionUndefinedError)
    }
}
