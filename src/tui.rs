use crate::app::{AppModeInfo, Display, Event};
use crate::notification;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind,
};
use futures::StreamExt;
use std::io;
use std::time::Duration;
use thiserror::Error;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets, Frame, Terminal,
};

pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    raw_mode_enabled: bool,
    alternate_screen_enabled: bool,
    event_stream: EventStream,
}

impl Tui {
    pub fn new() -> Result<Self, TuiError> {
        let backend = CrosstermBackend::new(io::stdout());

        Ok(Tui {
            terminal: Terminal::new(backend).map_err(TuiError::Creation)?,
            alternate_screen_enabled: false,
            raw_mode_enabled: false,
            event_stream: EventStream::new(),
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

    pub fn render(&mut self, render_data: &Display) -> Result<(), TuiError> {
        self.terminal
            .draw(|f| {
                render_ui(f, render_data);
            })
            .map_err(TuiError::Rendering)?;

        if render_data.notify_end_of_activity {
            // ignore errors for now, perhaps add a log message in the tui in the future
            let _ = notification::show_desktop_notification("", &render_data.activity.to_string());
        }

        Ok(())
    }

    pub async fn read_event(&mut self) -> Result<Event, TuiError> {
        loop {
            let crossterm_event = self.read_crossterm_event().await?;
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

fn render_ui(frame: &mut Frame<CrosstermBackend<io::Stdout>>, render_data: &Display) {
    let (_settings_chunk, timer_chunk) = {
        let toplevel_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(0), Constraint::Percentage(80)])
            .split(frame.size());

        (toplevel_chunks[0], toplevel_chunks[1])
    };

    let (_widget_settings_block, widget_timer_block) = {
        let base_block = widgets::Block::default().borders(widgets::Borders::ALL);
        let title_text_initial_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
        let title_text_base_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);

        (
            base_block.clone().title(Spans::from(vec![
                Span::styled("s", title_text_initial_style),
                Span::styled("ettings", title_text_base_style),
            ])),
            base_block.clone().title(Spans::from(vec![
                Span::styled("t", title_text_initial_style),
                Span::styled("imer", title_text_base_style),
            ])),
        )
    };

    let timer_chunk_within_border = {
        let horizontal = Layout::default()
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .direction(Direction::Horizontal)
            .split(timer_chunk);

        Layout::default()
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .direction(Direction::Vertical)
            .split(horizontal[1])[1]
    };

    let clock_animation_chunk = {
        let (clock_width, clock_height) = (21, 11); // TODO: make clock dimensions dynamic
        let (left_padding, right_padding);
        {
            let leftover_width = timer_chunk_within_border.width.saturating_sub(clock_width);
            left_padding = leftover_width / 2;
            right_padding = leftover_width.saturating_sub(left_padding);
        }
        let (top_padding, bottom_padding);
        {
            let leftover_height = timer_chunk_within_border
                .height
                .saturating_sub(clock_height);
            top_padding = leftover_height / 2;
            bottom_padding = leftover_height.saturating_sub(top_padding);
        }
        let vertically_centered_sub_chunk;
        {
            let vertical_sub_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(top_padding),
                    Constraint::Length(clock_height),
                    Constraint::Length(bottom_padding),
                ])
                .split(timer_chunk_within_border);
            vertically_centered_sub_chunk = vertical_sub_chunks[1];
        }
        let horizontal_sub_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_padding),
                Constraint::Length(clock_width),
                Constraint::Length(right_padding),
            ])
            .split(vertically_centered_sub_chunk);

        horizontal_sub_chunks[1]
    };

    let widget_clock_animation = {
        let partial_clock = animations::partial_box(1.0 - render_data.progress_percentage);

        widgets::Paragraph::new(partial_clock).alignment(Alignment::Left)
    };

    let clock_text = {
        let is_fourth_session = render_data.completed_focus_sessions % 4 == 0;
        let n_highlighted_indicators: usize = render_data.completed_focus_sessions as usize % 4
            + match (render_data.activity.is_focus(), is_fourth_session) {
                (true, _) => 1,
                (false, true) => 4,
                _ => 0,
            };

        format!(
            "{}\n{}\n{} {}",
            animations::completed_sessions_counter(n_highlighted_indicators, 4),
            render_data.time_remaining,
            render_data.activity,
            if render_data.timer_is_paused {
                "▶"
            } else {
                "⏸"
            }
        )
    };

    let clock_text_chunk = {
        let text_height = clock_text.lines().count() as u16;
        let ceil_padding = (clock_animation_chunk.height / 2).saturating_sub(text_height / 2);
        let floor_padding = clock_animation_chunk
            .height
            .saturating_sub(ceil_padding)
            .saturating_sub(text_height / 2);
        let vertical_sub_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(ceil_padding),
                Constraint::Length(text_height),
                Constraint::Length(floor_padding),
            ])
            .split(clock_animation_chunk);

        vertical_sub_chunks[1]
    };

    let widget_clock_text = widgets::Paragraph::new(clock_text).alignment(Alignment::Center);

    let network_info_text = {
        match &render_data.mode_info {
            AppModeInfo::Offline => "offline".to_string(),
            AppModeInfo::Server {
                connected_clients,
                listening_on,
            } => {
                let how_many_clients = match connected_clients.len() {
                    0 => "no clients".to_string(),
                    1 => "one client".to_string(),
                    n => format!("{n} clients"),
                };

                format!(
                    "listening on port {}\n{how_many_clients} connected",
                    listening_on.port()
                )
            }
            AppModeInfo::Client { connected_to } => {
                format!("connected to {connected_to}")
            }
        }
    };

    let widget_network_info = widgets::Paragraph::new(network_info_text).alignment(Alignment::Left);

    frame.render_widget(widget_network_info, timer_chunk_within_border);
    frame.render_widget(widget_clock_animation, clock_animation_chunk);
    frame.render_widget(widget_clock_text, clock_text_chunk);
    //frame.render_widget(_widget_settings_block, settings_chunk);
    frame.render_widget(widget_timer_block, timer_chunk);
}

mod animations {
    use itertools::intersperse;
    use std::iter;
    use unicode_segmentation::UnicodeSegmentation;

    pub fn completed_sessions_counter(
        n_highlighted_indicators: usize,
        n_indicators: usize,
    ) -> String {
        let counter = "▢".repeat(n_highlighted_indicators)
            + &"-".repeat(n_indicators.saturating_sub(n_highlighted_indicators));
        intersperse(counter.graphemes(true), " ").collect()
    }

    pub fn partial_box(percentage: f64) -> String {
        let percentage = percentage.max(0.0).min(1.0);

        const WHOLE_BOX: &str = "╭───────────────────╮
│                   │
│                   │
│                   │
│                   │
│                   │
│                   │
│                   │
│                   │
│                   │
╰───────────────────╯";

        const BOX_WIDTH: usize = 21;
        const BOX_HEIGHT: usize = 11;
        const N_BOX_ELEMENTS: usize = 60;

        let draw_n_bars = (N_BOX_ELEMENTS as f64 * percentage).ceil() as usize;
        let skip_n_bars = N_BOX_ELEMENTS - draw_n_bars;
        let mut grapheme_matrix: Vec<Vec<&str>> = WHOLE_BOX
            .lines()
            .map(|line| line.graphemes(true).collect())
            .collect();

        let mut path: Vec<(usize, usize)> = Vec::new();
        path.extend(iter::repeat(0).zip((0..BOX_WIDTH / 2).rev()));
        path.extend((1..BOX_HEIGHT).zip(iter::repeat(0)));
        path.extend(iter::repeat(BOX_HEIGHT - 1).zip(1..BOX_WIDTH));
        path.extend((0..BOX_HEIGHT - 1).rev().zip(iter::repeat(BOX_WIDTH - 1)));
        path.extend(iter::repeat(0).zip(((BOX_WIDTH - BOX_WIDTH / 2 - 1)..(BOX_WIDTH - 1)).rev()));

        for (row, col) in path.iter().take(skip_n_bars) {
            grapheme_matrix[*row][*col] = " ";
        }

        let result = grapheme_matrix.iter().fold(String::new(), |acc, vec| {
            acc + &vec
                .iter()
                .fold(String::new(), |acc: String, str| acc + *str)
                + "\n"
        });

        result
    }
}

pub struct EventConversionUndefinedError;

impl TryFrom<CrosstermEvent> for Event {
    type Error = EventConversionUndefinedError;

    fn try_from(value: CrosstermEvent) -> Result<Self, Self::Error> {
        match value {
            CrosstermEvent::Key(key_event)
                if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                match key_event.code {
                    KeyCode::Char('c') => Some(Event::Quit),
                    _ => None,
                }
            }
            CrosstermEvent::Key(key_event)
                if key_event.kind == KeyEventKind::Press
                    || key_event.kind == KeyEventKind::Repeat =>
            {
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
