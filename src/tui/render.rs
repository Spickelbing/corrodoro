use crate::app::NetworkStatus;
use crate::net::TimerVisuals;
use crate::tui::animation;
use crate::tui::widgets::{BlockWithLegend, Settings};
use std::io;
use tui::widgets::BorderType;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
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

        let toplevel_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(settings_pct),
                Constraint::Percentage(timer_pct),
            ])
            .split(frame.size());

        (toplevel_chunks[0], toplevel_chunks[1])
    };

    let widget_timer_block = define_block(
        "²timer",
        vec!["␣ toggle", "↕ adjust", "skip", "reset", "quit"],
    );

    let timer_chunk_within_border = widget_timer_block.inner(timer_chunk);

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
        let partial_clock = animation::clock(1.0 - timer_visuals.progress_percentage);

        widgets::Paragraph::new(partial_clock).alignment(Alignment::Left)
    };

    let clock_text = {
        let is_fourth_session = timer_visuals.completed_focus_sessions % 4 == 0;
        let n_highlighted_indicators: usize = timer_visuals.completed_focus_sessions as usize % 4
            + match (timer_visuals.activity.is_focus(), is_fourth_session) {
                (true, _) => 1,
                (false, true) => 4,
                _ => 0,
            };

        format!(
            "{}\n{}\n{} {}",
            animation::session_counter(n_highlighted_indicators, 4),
            timer_visuals.time_remaining,
            timer_visuals.activity,
            if timer_visuals.timer_is_paused {
                "⏵"
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

    let widget_network_info = Settings::default()
        .status(&network_info_text)
        .block(define_block("¹settings", vec![]));

    if show_settings {
        frame.render_widget(widget_network_info, settings_chunk);
    }
    if show_timer {
        frame.render_widget(widget_clock_animation, clock_animation_chunk);
        frame.render_widget(widget_clock_text, clock_text_chunk);
        frame.render_widget(widget_timer_block, timer_chunk);
    }
}
