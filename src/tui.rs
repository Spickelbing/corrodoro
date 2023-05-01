use crate::event::{Event, EventConversionUndefinedError};
use crate::pomodoro::State as PomodoroState;
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyModifiers, MouseEventKind};
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::io;
use std::time::Duration;
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
}

impl Tui {
    pub fn new() -> Result<Self, TuiError> {
        let backend = CrosstermBackend::new(io::stdout());

        Ok(Tui {
            terminal: Terminal::new(backend).map_err(TuiError::Creation)?,
            alternate_screen_enabled: false,
            raw_mode_enabled: false,
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
            crossterm::event::EnableMouseCapture
        )
        .map_err(TuiError::AlternateScreenToggle)?;
        self.alternate_screen_enabled = true;

        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), TuiError> {
        if self.alternate_screen_enabled {
            crossterm::execute!(
                self.terminal.backend_mut(),
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

    pub fn render(&mut self, display_data: &DisplayData) -> Result<(), TuiError> {
        self.terminal
            .draw(|f| {
                render_ui(f, display_data);
            })
            .map_err(TuiError::Rendering)?;

        Ok(())
    }

    pub fn try_read_events(&self, max_events: u32) -> Result<Vec<Event>, TuiError> {
        let crossterm_events = self.try_read_crossterm_events(max_events)?;
        Ok(crossterm_events
            .into_iter()
            .filter_map(|e| e.try_into().ok())
            .collect())
    }

    fn try_read_crossterm_events(&self, max_events: u32) -> Result<Vec<CrosstermEvent>, TuiError> {
        let mut events = vec![];
        let mut events_read = 0;

        while crossterm::event::poll(Duration::from_secs(0)).map_err(TuiError::ReadInputEvent)? {
            if events_read == max_events {
                break;
            }
            events.push(crossterm::event::read().map_err(TuiError::ReadInputEvent)?);
            events_read += 1;
        }

        Ok(events)
    }
}

pub struct DisplayData {
    pub timer_text: String,
    pub activity_name: String,
    pub progress_percentage: f64,
    pub is_paused: bool,
}

#[derive(Debug)]
pub enum TuiError {
    Creation(io::Error),
    RawModeToggle(io::Error),
    AlternateScreenToggle(io::Error),
    Rendering(io::Error),
    ReadInputEvent(io::Error),
}

impl Display for TuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TuiError::Creation(error) => {
                write!(f, "failed to initialize terminal ui: {error}")
            }
            TuiError::RawModeToggle(error) => {
                write!(f, "failed to toggle terminal raw mode: {error}")
            }
            TuiError::AlternateScreenToggle(error) => {
                write!(f, "failed to toggle alternate terminal screen: {error}")
            }
            TuiError::Rendering(error) => write!(f, "failed to render terminal ui: {error}"),
            TuiError::ReadInputEvent(error) => {
                write!(f, "failed to read input event from terminal: {error}")
            }
        }
    }
}

impl Error for TuiError {}

fn render_ui(frame: &mut Frame<CrosstermBackend<io::Stdout>>, display_data: &DisplayData) {
    let (_settings_chunk, timer_chunk);
    {
        let toplevel_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(0), Constraint::Percentage(80)])
            .split(frame.size());
        (_settings_chunk, timer_chunk) = (toplevel_chunks[0], toplevel_chunks[1]);
    }

    let timer_clock_sub_chunk;
    {
        let (clock_width, clock_height) = (21, 11); // TODO: make dimensions dynamic
        let (left_padding, right_padding);
        {
            let leftover_width = timer_chunk.width.saturating_sub(clock_width);
            left_padding = leftover_width / 2;
            right_padding = leftover_width.saturating_sub(left_padding);
        }
        let (top_padding, bottom_padding);
        {
            let leftover_height = timer_chunk.height.saturating_sub(clock_height);
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
                .split(timer_chunk);
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
        timer_clock_sub_chunk = horizontal_sub_chunks[1];
    }

    let timer_text_sub_chunk;
    {
        let text_height = 2; // TODO: make this dynamic
        let ceil_padding = timer_clock_sub_chunk.height / 2;
        let floor_padding = timer_clock_sub_chunk
            .height
            .saturating_sub(ceil_padding)
            .saturating_sub(text_height);
        let vertical_sub_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(ceil_padding),
                Constraint::Length(text_height),
                Constraint::Length(floor_padding),
            ])
            .split(timer_clock_sub_chunk);
        timer_text_sub_chunk = vertical_sub_chunks[1];
    }

    let (_widget_settings_block, widget_timer_block);
    {
        let base_block = widgets::Block::default()
            .borders(widgets::Borders::ALL)
            .style(Style::default().bg(Color::Black));
        let title_text_initial_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
        let title_text_base_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        _widget_settings_block = base_block.clone().title(Spans::from(vec![
            Span::styled("s", title_text_initial_style),
            Span::styled("ettings", title_text_base_style),
        ]));
        widget_timer_block = base_block.clone().title(Spans::from(vec![
            Span::styled("t", title_text_initial_style),
            Span::styled("imer", title_text_base_style),
        ]));
    }

    let timer_text = format!(
        "{}\n{} {}",
        display_data.timer_text,
        display_data.activity_name,
        if display_data.is_paused { "▶" } else { "⏸" }
    );
    let widget_timer_text = widgets::Paragraph::new(timer_text).alignment(Alignment::Center);

    let widget_clock_animation;
    {
        let partial_clock = animations::partial_box(1.0 - display_data.progress_percentage);
        widget_clock_animation = widgets::Paragraph::new(partial_clock).alignment(Alignment::Left);
    }

    frame.render_widget(widget_clock_animation, timer_clock_sub_chunk);
    frame.render_widget(widget_timer_text, timer_text_sub_chunk);
    //frame.render_widget(widget_settings_block, settings_chunk);
    frame.render_widget(widget_timer_block, timer_chunk);
}

mod animations {
    use unicode_segmentation::UnicodeSegmentation;

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
        path.extend(
            [0usize]
                .repeat(BOX_WIDTH / 2)
                .into_iter()
                .zip((0..BOX_WIDTH / 2).rev()),
        );
        path.extend((1..BOX_HEIGHT).zip([0].repeat(BOX_HEIGHT - 1)));
        path.extend(
            [BOX_HEIGHT - 1]
                .repeat(BOX_WIDTH - 1)
                .into_iter()
                .zip(1..BOX_WIDTH),
        );
        path.extend(
            (0..BOX_HEIGHT - 1)
                .rev()
                .zip([BOX_WIDTH - 1].repeat(BOX_HEIGHT - 1)),
        );
        path.extend(
            [0].repeat(BOX_WIDTH - BOX_WIDTH / 2)
                .into_iter()
                .zip((BOX_WIDTH - BOX_WIDTH / 2 - 1)..(BOX_WIDTH - 1))
                .rev(),
        );

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
            CrosstermEvent::Key(key_event) => match key_event.code {
                KeyCode::Char('q') => Some(Event::Quit),
                KeyCode::Char('r') => Some(Event::ResetTimer),
                KeyCode::Char('s') => Some(Event::SkipActivity),
                KeyCode::Char(' ') => Some(Event::ToggleTimer),
                KeyCode::Up => Some(Event::ExtendActivity(Duration::from_secs(60))),
                KeyCode::Down => Some(Event::ReduceActivity(Duration::from_secs(60))),
                KeyCode::Esc => Some(Event::Quit),
                _ => None,
            },
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

impl From<&PomodoroState> for DisplayData {
    fn from(state: &PomodoroState) -> Self {
        Self {
            activity_name: state.current_activity().to_string(),
            progress_percentage: state.progress_percentage(),
            timer_text: state.time_remaining().to_string(),
            is_paused: !state.timer_is_active(),
        }
    }
}
