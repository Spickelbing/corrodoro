use std::iter::once;
use tui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap},
};

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

    pub fn status(mut self, status: &'a str) -> Settings<'a> {
        self.network_status = status;
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
