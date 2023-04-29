use unicode_segmentation::UnicodeSegmentation;

pub fn _partial_unicode_circle(percentage: f64) -> String {
    let percentage = percentage.max(0.0).min(1.0);

    const WHOLE_CIRCLE: &str = "            ▄▄▄▄▄▄▄
       ▄▄▀▀▀       ▀▀▀▄▄
     ▄▀                 ▀▄
   ▄▀                     ▀▄
  █                         █
 █                           █
▄▀                           ▀▄
█                             █
█                             █
 █                           █
 ▀▄                         ▄▀
  ▀▄                       ▄▀
    ▀▄                   ▄▀
      ▀▄▄             ▄▄▀
         ▀▀▀▄▄▄▄▄▄▄▀▀▀";

    const GRAPHEME_REPLACEMENTS: [(usize, &str); 80] = [
        (15, "▗"),
        (14, " "),
        (13, " "),
        (12, " "),
        (31, " "),
        (30, " "),
        (29, " "),
        (28, " "),
        (27, " "),
        (51, " "),
        (50, " "),
        (76, " "),
        (75, " "),
        (103, "▄"),
        (103, " "),
        (132, "▄"),
        (132, " "),
        (163, " "),
        (162, " "),
        (194, "▄"),
        (194, " "),
        (226, "▄"),
        (226, " "),
        (259, "▄"),
        (259, " "),
        (290, " "),
        (291, " "),
        (322, " "),
        (323, " "),
        (354, " "),
        (355, " "),
        (384, " "),
        (385, " "),
        (386, " "),
        (413, " "),
        (414, " "),
        (415, " "),
        (416, " "),
        (417, " "),
        (418, " "),
        (419, " "),
        (420, " "),
        (421, " "),
        (422, " "),
        (423, " "),
        (424, " "),
        (425, " "),
        (400, " "),
        (401, " "),
        (402, " "),
        (375, " "),
        (376, " "),
        (347, " "),
        (348, " "),
        (317, " "),
        (318, " "),
        (287, "▀"),
        (287, " "),
        (256, "▀"),
        (256, " "),
        (224, "▀"),
        (224, " "),
        (192, " "),
        (191, " "),
        (160, "▀"),
        (160, " "),
        (129, "▀"),
        (129, " "),
        (99, " "),
        (98, " "),
        (70, " "),
        (69, " "),
        (43, " "),
        (42, " "),
        (41, " "),
        (40, " "),
        (39, " "),
        (18, " "),
        (17, " "),
        (16, " "),
    ];

    let take_n = (GRAPHEME_REPLACEMENTS.len() as f64 * percentage) as usize;
    let mut graphemes: Vec<&str> = WHOLE_CIRCLE.graphemes(true).collect();

    for (replace_i, replace_s) in GRAPHEME_REPLACEMENTS.iter().take(take_n) {
        graphemes[*replace_i] = replace_s;
    }

    graphemes.into_iter().collect()
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

    let draw_n_bars = (N_BOX_ELEMENTS as f64 * percentage) as usize;
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
