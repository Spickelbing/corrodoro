use crate::pomodoro;
use crate::{AppError, CloseThreadNotificiation};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyModifiers, MouseEventKind};
use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets, Frame, Terminal,
};

pub fn ui_thread(
    ui_rx: mpsc::Receiver<pomodoro::State>,
    events_tx: mpsc::Sender<Event>,
    close_rx: mpsc::Receiver<CloseThreadNotificiation>,
) -> Result<(), AppError> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let update_interval = std::time::Duration::from_millis(100);

    let mut return_val = Ok(());
    let mut state: Option<pomodoro::State> = None;
    loop {
        let now = Instant::now();

        match close_rx.try_recv() {
            Ok(_) => {
                break;
            }
            Err(error @ mpsc::TryRecvError::Disconnected) => {
                return_val = Err(AppError::from(error));
                break;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }

        // TODO: handle hangup
        if let Some(new_state) = ui_rx.try_iter().last() {
            state = Some(new_state);
        }
        // let ui_state = ...; // for highlighting current setting selection, etc.

        for event in try_read_crossterm_events(10)? {
            events_tx.send(event)?;
        }

        terminal.draw(|f| {
            draw_ui(f, &state);
        })?;

        if now.elapsed() < update_interval {
            std::thread::sleep(update_interval - now.elapsed());
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

fn draw_ui(frame: &mut Frame<CrosstermBackend<io::Stdout>>, state: &Option<pomodoro::State>) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(frame.size());

    let base_block = widgets::Block::default()
        .borders(widgets::Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let title_text_initial_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
    let title_text_base_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let left_block = base_block.clone().title(Spans::from(vec![
        Span::styled("s", title_text_initial_style),
        Span::styled("ettings", title_text_base_style),
    ]));

    let right_block = base_block.clone().title(Spans::from(vec![
        Span::styled("t", title_text_initial_style),
        Span::styled("imer", title_text_base_style),
    ]));

    let timer_text = match state {
        Some(state) => format!("{}", state.time_remaining()),
        None => String::from("no timer state available"),
    };

    let right_block = widgets::Paragraph::new(timer_text)
        .block(right_block)
        .alignment(Alignment::Center);

    frame.render_widget(left_block, chunks[0]);
    frame.render_widget(right_block, chunks[1]);
}

/// represents a user-input event, independent of the UI
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

fn try_read_crossterm_events(max_events: u32) -> io::Result<Vec<Event>> {
    let mut translated_events = vec![];

    let mut i = 0;
    while crossterm::event::poll(Duration::from_secs(0))? {
        if i == max_events {
            break;
        }

        let event = crossterm::event::read()?;

        if let Ok(translated_event) = Event::try_from(event) {
            translated_events.push(translated_event);
        }

        i += 1;
    }

    Ok(translated_events)
}
