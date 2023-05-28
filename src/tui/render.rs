use crate::app::NetworkStatus;
use crate::net::TimerVisuals;
use crate::tui::widgets::{BlockWithLegend, PomodoroClock, Settings};
use std::io;
use tui::widgets::BorderType;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets, Frame,
};
use unicode_segmentation::UnicodeSegmentation;

fn split_initial(str: &str) -> (&str, &str) {
    let mut graphemes = str.graphemes(true);

    let initial = graphemes.next().unwrap_or("");
    let remainder = graphemes.as_str();

    (initial, remainder)
}

fn define_block<'a>(title: &'a str, legend: Vec<&'a str>) -> BlockWithLegend<'a> {
    let (initial, remainder) = split_initial(title);

    let text_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let initials_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

    let title = Spans::from(vec![
        Span::styled(initial, initials_style),
        Span::styled(remainder, text_style),
    ]);

    let legend = legend
        .into_iter()
        .map(|s| {
            let (initial, remainder) = split_initial(s);

            Spans::from(vec![
                Span::styled(initial, initials_style),
                Span::styled(remainder, text_style),
            ])
        })
        .collect();

    BlockWithLegend::default()
        .borders(widgets::Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .legend(legend)
}

pub fn render_ui(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    timer_visuals: &TimerVisuals,
    network_status: &NetworkStatus,
    show_settings: bool,
    show_timer: bool,
) {
    let (settings_chunk, timer_chunk) = {
        let (settings_pct, timer_pct) = match (show_settings, show_timer) {
            (true, true) => (20, 80),
            (true, false) => (100, 0),
            (false, true) => (0, 100),
            (false, false) => (0, 0),
        };

        let direction = if frame.size().width >= frame.size().height * 2 {
            Direction::Horizontal
        } else {
            Direction::Vertical
        };

        let toplevel_chunks = Layout::default()
            .direction(direction)
            .constraints([
                Constraint::Percentage(settings_pct),
                Constraint::Percentage(timer_pct),
            ])
            .split(frame.size());

        (toplevel_chunks[0], toplevel_chunks[1])
    };

    if show_settings {
        let network_info_text = {
            match &network_status {
                NetworkStatus::Offline => "offline".to_string(),
                NetworkStatus::Server {
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
                NetworkStatus::Client { connected_to } => {
                    format!("connected to {connected_to}")
                }
            }
        };

        let settings_widget = Settings::default()
            .network_status(&network_info_text)
            .block(define_block("¹settings", vec![]));

        frame.render_widget(settings_widget, settings_chunk);
    }
    if show_timer {
        let is_fourth_session = timer_visuals.completed_focus_sessions % 4 == 0;
        let n_highlighted_indicators = timer_visuals.completed_focus_sessions as u8 % 4
            + match (timer_visuals.activity.is_focus(), is_fourth_session) {
                (true, _) => 1,
                (false, true) => 4,
                _ => 0,
            };

        let timer_widget = PomodoroClock::default()
            .block(define_block(
                "²timer",
                vec!["␣ toggle", "↕ adjust", "skip", "reset", "quit"],
            ))
            .break_counter_total(4)
            .break_counter_filled(n_highlighted_indicators)
            .completed_focus_sessions(timer_visuals.completed_focus_sessions)
            .duration(timer_visuals.time_remaining)
            .timer_is_paused(timer_visuals.timer_is_paused)
            .progress_percentage(timer_visuals.progress_percentage);

        frame.render_widget(timer_widget, timer_chunk);
    }
}
