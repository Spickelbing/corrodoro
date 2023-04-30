use crate::animations;
use crate::pomodoro;
use crate::{AppError, CloseThreadNotificiation};
use crossbeam::{select, channel::{tick, Receiver, Sender}};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyModifiers, MouseEventKind};
use std::io;
use std::time::Duration;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets, Frame, Terminal,
};

pub fn ui_thread(
    ui_rx: Receiver<pomodoro::State>,
    events_tx: Sender<Event>,
    close_rx: Receiver<CloseThreadNotificiation>,
) -> Result<(), AppError> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let crossterm_poll_interval = Duration::from_millis(20);
    let crossterm_poll_ticker_rx = tick(crossterm_poll_interval);

    let return_val;
    let mut state: Option<pomodoro::State> = None;
    loop {
        terminal.draw(|f| {
            draw_ui(f, &state);
        })?;

        select! {
            recv(close_rx) -> close => {
                match close {
                    Ok(_) => return_val = Ok(()),
                    Err(error) => return_val = Err(AppError::from(error)),
                }
                break;
            }
            recv(ui_rx) -> new_state => {
                match new_state {
                    Ok(new_state) => state = Some(new_state),
                    Err(error) => {
                        return_val = Err(AppError::from(error));
                        break;
                    },
                }
            }
            recv(crossterm_poll_ticker_rx) -> instant => {
                instant?;
                for crossterm_event in try_read_crossterm_events(10)? {
                    if let Ok(event) = Event::try_from(crossterm_event) {
                        events_tx.send(event)?;
                    }
                }
            }
        }
    }

    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
    )?;
    crossterm::terminal::disable_raw_mode()?;

    return_val
}

#[derive(Default)]
struct ApproximateLayout {
    settings_widget: Rect,
    timer_widget: Rect,
}

fn draw_ui(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    state: &Option<pomodoro::State>,
) -> ApproximateLayout {
    let (settings_chunk, timer_chunk);
    {
        let toplevel_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(frame.size());
        (settings_chunk, timer_chunk) = (toplevel_chunks[0], toplevel_chunks[1]);
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

    let (widget_settings_block, widget_timer_block);
    {
        let base_block = widgets::Block::default()
            .borders(widgets::Borders::ALL)
            .style(Style::default().bg(Color::Black));
        let title_text_initial_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
        let title_text_base_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        widget_settings_block = base_block.clone().title(Spans::from(vec![
            Span::styled("s", title_text_initial_style),
            Span::styled("ettings", title_text_base_style),
        ]));
        widget_timer_block = base_block.clone().title(Spans::from(vec![
            Span::styled("t", title_text_initial_style),
            Span::styled("imer", title_text_base_style),
        ]));
    }

    let widget_timer_text;
    {
        let timer_text = match state {
            Some(state) => format!("{state}"),
            None => String::from("no timer state available"),
        };
        widget_timer_text = widgets::Paragraph::new(timer_text).alignment(Alignment::Center);
    }

    let widget_clock_text_art;
    {
        let progress_percentage = match state {
            Some(state) => state.progress_percentage(),
            None => 1.0,
        };
        let clock_text_art = animations::partial_box(1.0 - progress_percentage);
        widget_clock_text_art = widgets::Paragraph::new(clock_text_art).alignment(Alignment::Left);
    }

    frame.render_widget(widget_clock_text_art, timer_clock_sub_chunk);
    frame.render_widget(widget_timer_text, timer_text_sub_chunk);
    frame.render_widget(widget_settings_block, settings_chunk);
    frame.render_widget(widget_timer_block, timer_chunk);

    ApproximateLayout {
        settings_widget: settings_chunk,
        timer_widget: timer_chunk,
    }
}

/// Represents a user-input event, independent of the UI.
pub enum Event {
    Quit,
    ToggleTimer,
    ResetTimer,
    SkipActivity,
    ExtendActivity(Duration),
    ReduceActivity(Duration),
}

pub struct EventConversionError;

impl TryFrom<CrosstermEvent> for Event {
    type Error = EventConversionError;

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
        .ok_or(EventConversionError)
    }
}

fn try_read_crossterm_events(max_events: u32) -> Result<Vec<CrosstermEvent>, io::Error> {
    let mut events = vec![];
    let mut events_read = 0;

    while crossterm::event::poll(Duration::from_secs(0))? {
        if events_read == max_events {
            break;
        }
        events.push(crossterm::event::read()?);
        events_read += 1;
    }

    Ok(events)
}
