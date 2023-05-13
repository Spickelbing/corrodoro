use crate::app::AppModeInfo;
use crate::app::Display;
use crate::tui::animation;
use std::io;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets,
    Frame,
};

fn define_block<'a>(title: &'a str, legend: Vec<&'a str>) -> widgets::Block<'a> {
    let text_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let initials_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

    let (initial, remainder) = match title.len() {
        0 => ("", ""),
        1 => (title, ""),
        _ => title.split_at(1),
    };

    let title = Spans::from(vec![
        Span::styled(initial, initials_style),
        Span::styled(remainder, text_style),
    ]);

    widgets::Block::default()
        .borders(widgets::Borders::ALL)
        .title(title)
}

pub fn render_ui(frame: &mut Frame<CrosstermBackend<io::Stdout>>, render_data: &Display) {
    let (_settings_chunk, timer_chunk) = {
        let toplevel_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(0), Constraint::Percentage(80)])
            .split(frame.size());

        (toplevel_chunks[0], toplevel_chunks[1])
    };

    let widget_timer_block = define_block("timer", vec![]);

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
        let partial_clock = animation::clock(1.0 - render_data.progress_percentage);

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
            animation::session_counter(n_highlighted_indicators, 4),
            render_data.time_remaining,
            render_data.activity,
            if render_data.timer_is_paused {
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
    frame.render_widget(widget_timer_block, timer_chunk);
}
