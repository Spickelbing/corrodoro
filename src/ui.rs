use crate::state::{Event, PomodoroState};
use crate::ThreadError;
use crossterm;
use std::io;
use std::sync::mpsc;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets, Frame, Terminal,
};

pub enum UiUpdate {
    StateUpdate(PomodoroState),
    TerminalEvent(crossterm::event::Event),
}

pub fn ui_thread(
    ui_tx: mpsc::Sender<UiUpdate>,
    ui_rx: mpsc::Receiver<UiUpdate>,
    events_tx: mpsc::Sender<Event>,
) -> Result<(), ThreadError> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let event_handler_thread = std::thread::spawn(move || {
        event_handler_thread(ui_tx);
    });

    let mut state: Option<PomodoroState> = None;
    loop {
        match ui_rx.recv()? {
            UiUpdate::StateUpdate(new_state) => {
                state = Some(new_state);
            }
            UiUpdate::TerminalEvent(event) => match event {
                crossterm::event::Event::Resize(_, _) => (),
                _ => continue,
            },
        };
        // let ui_state = ...; // for highlighting current setting selection, etc.

        terminal.draw(|f| {
            draw_ui(f, &state);
        })?;
    }

    event_handler_thread.join().unwrap(); // TODO: handle error

    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
    )?;
    crossterm::terminal::disable_raw_mode()?;
}

fn event_handler_thread(
    ui_tx : mpsc::Sender<UiUpdate>,
) -> Result<(), ThreadError> {
    loop {
        let event = crossterm::event::read()?;
        // TODO: stop this thread on events that mean exit
        ui_tx.send(UiUpdate::TerminalEvent(event))?;
    }
}

fn draw_ui(frame: &mut Frame<CrosstermBackend<io::Stdout>>, state: &Option<PomodoroState>) {
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
        Span::styled("s", title_text_initial_style.clone()),
        Span::styled("ettings", title_text_base_style.clone()),
    ]));

    let right_block = base_block.clone().title(Spans::from(vec![
        Span::styled("t", title_text_initial_style.clone()),
        Span::styled("imer", title_text_base_style.clone()),
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
