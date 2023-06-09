use crate::pomodoro::{Activity, SessionDuration};
use crate::tui::animation;
use std::iter::once;
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;

pub struct BlockWithLegend<'a> {
    legend: Vec<Spans<'a>>,
    block: Block<'a>,
    border_type: BorderType,
}

impl<'a> BlockWithLegend<'a> {
    pub fn title<T>(mut self, title: T) -> BlockWithLegend<'a>
    where
        T: Into<Spans<'a>>,
    {
        self.block = self.block.title(title);
        self
    }

    #[allow(dead_code)]
    pub fn title_alignment(mut self, alignment: Alignment) -> BlockWithLegend<'a> {
        self.block = self.block.title_alignment(alignment);
        self
    }

    #[allow(dead_code)]
    pub fn border_style(mut self, style: Style) -> BlockWithLegend<'a> {
        self.block = self.block.border_style(style);
        self
    }

    #[allow(dead_code)]
    pub fn style(mut self, style: Style) -> BlockWithLegend<'a> {
        self.block = self.block.style(style);
        self
    }

    pub fn borders(mut self, borders: Borders) -> BlockWithLegend<'a> {
        self.block = self.block.borders(borders);
        self
    }

    #[allow(dead_code)]
    pub fn border_type(mut self, border_type: BorderType) -> BlockWithLegend<'a> {
        self.block = self.block.border_type(border_type);
        self.border_type = border_type;
        self
    }

    #[allow(dead_code)]
    pub fn inner(&self, inner: Rect) -> Rect {
        self.block.inner(inner)
    }

    pub fn legend<T>(mut self, legend: Vec<T>) -> BlockWithLegend<'a>
    where
        T: Into<Spans<'a>>,
    {
        self.legend = legend.into_iter().map(|l| l.into()).collect();
        self
    }
}

impl<'a> Default for BlockWithLegend<'a> {
    fn default() -> BlockWithLegend<'a> {
        BlockWithLegend {
            legend: vec![],
            block: Block::default(),
            border_type: BorderType::Plain,
        }
    }
}

impl<'a> Widget for BlockWithLegend<'a> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        self.block.render(area, buf);
        let symbols = BorderType::line_symbols(self.border_type);

        let legend_y = area.y + area.height - 1;
        let mut legend_x = area.x + 1;

        for legend in self.legend.into_iter() {
            let legend: Spans = once(Span::from(symbols.bottom_right))
                .chain(legend.0.into_iter())
                .chain(once(Span::from(symbols.bottom_left)))
                .collect::<Vec<_>>()
                .into();

            let width_remaining = area.width.saturating_sub(legend_x - area.x + 1);
            let (x, _) = buf.set_spans(legend_x, legend_y, &legend, width_remaining);
            legend_x = x;
        }
    }
}

pub struct PomodoroClock<'a> {
    block: Option<BlockWithLegend<'a>>,
    completed_focus_sessions: u32,
    break_counter_filled: u8,
    break_counter_total: u8,
    progress_percentage: f64,
    duration: SessionDuration,
    activity: Activity,
    is_paused: bool,
}

impl<'a> PomodoroClock<'a> {
    pub fn block(mut self, block: BlockWithLegend<'a>) -> PomodoroClock<'a> {
        self.block = Some(block);
        self
    }

    pub fn new(
        completed_focus_sessions: u32,
        break_counter_filled: u8,
        break_counter_total: u8,
        progress_percentage: f64,
        duration: SessionDuration,
        activity: Activity,
        is_paused: bool,
    ) -> Self {
        PomodoroClock {
            completed_focus_sessions,
            break_counter_filled,
            break_counter_total,
            progress_percentage,
            duration,
            activity,
            is_paused,
            ..Default::default()
        }
    }
}

impl<'a> Widget for PomodoroClock<'a> {
    fn render(self, area: Rect, buf: &mut tui::buffer::Buffer) {
        let inner_area = match &self.block {
            Some(block) => block.inner(area),
            None => area,
        };
        if let Some(block) = self.block {
            block.render(area, buf);
        }

        let clock = animation::clock(1.0 - self.progress_percentage);
        let clock_height = clock.lines().count() as u16;
        let clock_width = clock
            .lines()
            .map(|l| l.graphemes(true).count())
            .max()
            .unwrap_or_default() as u16;

        let centered_chunk = {
            let (left_padding, right_padding);
            {
                let leftover_width = inner_area.width.saturating_sub(clock_width);
                left_padding = leftover_width / 2;
                right_padding = leftover_width.saturating_sub(left_padding);
            }
            let (top_padding, bottom_padding);
            {
                let leftover_height = inner_area.height.saturating_sub(clock_height);
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
                    .split(inner_area);
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

        if centered_chunk.height < clock_height || centered_chunk.width < clock_width {
            InsufficientSpaceWarning::new(
                (centered_chunk.width, centered_chunk.height),
                (clock_width, clock_height),
            )
            .render(inner_area, buf);
            return;
        }

        Paragraph::new(clock)
            .alignment(Alignment::Left)
            .render(centered_chunk, buf);

        let status_text = {
            format!(
                "{}\n{}\n{} {}",
                animation::session_counter(
                    self.break_counter_filled.into(),
                    self.break_counter_total.into()
                ),
                self.duration,
                self.activity,
                if self.is_paused { "⏵" } else { "⏸" }
            )
        };

        let text_chunk = centered_chunk.inner(&Margin {
            horizontal: 1,
            vertical: 1,
        });

        let text_height = status_text.lines().count() as u16;
        let ceil_padding = (text_chunk.height / 2).saturating_sub(text_height / 2);

        let text_chunk = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(ceil_padding), Constraint::Min(0)])
            .split(text_chunk)[1];

        Paragraph::new(status_text)
            .alignment(Alignment::Center)
            .render(text_chunk, buf);
    }
}

impl<'a> Default for PomodoroClock<'a> {
    fn default() -> PomodoroClock<'a> {
        PomodoroClock {
            block: None,
            completed_focus_sessions: 0,
            activity: Activity::Focus,
            break_counter_filled: 0,
            break_counter_total: 4,
            progress_percentage: 0.0,
            duration: SessionDuration::default(),
            is_paused: true,
        }
    }
}

#[derive(Default)]
pub struct Settings<'a> {
    block: Option<BlockWithLegend<'a>>,
    network_status: &'a str,
}

impl<'a> Settings<'a> {
    pub fn block(mut self, block: BlockWithLegend<'a>) -> Settings<'a> {
        self.block = Some(block);
        self
    }

    pub fn network_status(mut self, network_status: &'a str) -> Settings<'a> {
        self.network_status = network_status;
        self
    }
}

impl<'a> Widget for Settings<'a> {
    fn render(self, area: Rect, buf: &mut tui::buffer::Buffer) {
        let inner_area = match &self.block {
            Some(block) => block.inner(area),
            None => area,
        };
        if let Some(block) = self.block {
            block.render(area, buf);
        }

        let text_area = {
            let text_height = self.network_status.lines().count() as u16;
            let top_padding = (inner_area.height / 2).saturating_sub(text_height / 2);

            Layout::default()
                .constraints([
                    Constraint::Length(top_padding),
                    Constraint::Min(text_height),
                ])
                .split(inner_area)[1]
        };

        Paragraph::new(self.network_status)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(text_area, buf);
    }
}

#[derive(Default)]
struct InsufficientSpaceWarning {
    have: (u16, u16),
    need: (u16, u16),
}

impl InsufficientSpaceWarning {
    fn new(have: (u16, u16), need: (u16, u16)) -> Self {
        InsufficientSpaceWarning { have, need }
    }
}

impl Widget for InsufficientSpaceWarning {
    fn render(self, area: Rect, buf: &mut tui::buffer::Buffer) {
        let text = format!(
            "terminal size too small\nhave {}×{}\nneed {}×{}",
            self.have.0, self.have.1, self.need.0, self.need.1
        );
        let top_padding = (area.height / 2).saturating_sub(text.lines().count() as u16 / 2);

        let vertical_center = Layout::default()
            .constraints([Constraint::Length(top_padding), Constraint::Min(0)])
            .split(area)[1];

        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(vertical_center, buf);
    }
}
