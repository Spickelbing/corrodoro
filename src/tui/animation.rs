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
