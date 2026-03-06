use ratatui::style::Color;
use std::sync::atomic::{AtomicUsize, Ordering};

static THEME_INDEX: AtomicUsize = AtomicUsize::new(0);

pub const THEME_NAMES: &[&str] = &[
    "Catppuccin Mocha",
    "Tokyo Night",
    "Dracula",
    "Gruvbox Dark",
    "Nord",
];

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub rosewater: Color,
    pub flamingo: Color,
    pub pink: Color,
    pub mauve: Color,
    pub red: Color,
    pub maroon: Color,
    pub peach: Color,
    pub yellow: Color,
    pub green: Color,
    pub teal: Color,
    pub sky: Color,
    pub sapphire: Color,
    pub blue: Color,
    pub lavender: Color,
    pub text: Color,
    pub subtext1: Color,
    pub subtext0: Color,
    pub overlay2: Color,
    pub overlay1: Color,
    pub overlay0: Color,
    pub surface2: Color,
    pub surface1: Color,
    pub surface0: Color,
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,
}

const THEMES: &[Theme] = &[
    // Catppuccin Mocha
    Theme {
        rosewater: Color::Rgb(245, 224, 220),
        flamingo: Color::Rgb(242, 205, 205),
        pink: Color::Rgb(245, 194, 231),
        mauve: Color::Rgb(203, 166, 247),
        red: Color::Rgb(243, 139, 168),
        maroon: Color::Rgb(235, 160, 172),
        peach: Color::Rgb(250, 179, 135),
        yellow: Color::Rgb(249, 226, 175),
        green: Color::Rgb(166, 227, 161),
        teal: Color::Rgb(148, 226, 213),
        sky: Color::Rgb(137, 220, 235),
        sapphire: Color::Rgb(116, 199, 236),
        blue: Color::Rgb(137, 180, 250),
        lavender: Color::Rgb(180, 190, 254),
        text: Color::Rgb(205, 214, 244),
        subtext1: Color::Rgb(186, 194, 222),
        subtext0: Color::Rgb(166, 173, 200),
        overlay2: Color::Rgb(147, 153, 178),
        overlay1: Color::Rgb(127, 132, 156),
        overlay0: Color::Rgb(108, 112, 134),
        surface2: Color::Rgb(88, 91, 112),
        surface1: Color::Rgb(69, 71, 90),
        surface0: Color::Rgb(49, 50, 68),
        base: Color::Rgb(30, 30, 46),
        mantle: Color::Rgb(24, 24, 37),
        crust: Color::Rgb(17, 17, 27),
    },
    // Tokyo Night
    Theme {
        rosewater: Color::Rgb(216, 175, 175),
        flamingo: Color::Rgb(199, 160, 160),
        pink: Color::Rgb(255, 121, 198),
        mauve: Color::Rgb(187, 154, 247),
        red: Color::Rgb(247, 118, 142),
        maroon: Color::Rgb(219, 148, 148),
        peach: Color::Rgb(255, 158, 100),
        yellow: Color::Rgb(224, 175, 104),
        green: Color::Rgb(158, 206, 106),
        teal: Color::Rgb(115, 218, 202),
        sky: Color::Rgb(125, 207, 255),
        sapphire: Color::Rgb(42, 195, 222),
        blue: Color::Rgb(122, 162, 247),
        lavender: Color::Rgb(155, 165, 210),
        text: Color::Rgb(192, 202, 245),
        subtext1: Color::Rgb(169, 177, 214),
        subtext0: Color::Rgb(145, 152, 187),
        overlay2: Color::Rgb(120, 127, 162),
        overlay1: Color::Rgb(96, 103, 137),
        overlay0: Color::Rgb(72, 79, 112),
        surface2: Color::Rgb(59, 66, 97),
        surface1: Color::Rgb(52, 59, 88),
        surface0: Color::Rgb(41, 46, 66),
        base: Color::Rgb(26, 27, 38),
        mantle: Color::Rgb(22, 22, 30),
        crust: Color::Rgb(18, 18, 24),
    },
    // Dracula
    Theme {
        rosewater: Color::Rgb(255, 241, 224),
        flamingo: Color::Rgb(255, 183, 197),
        pink: Color::Rgb(255, 121, 198),      // dracula pink
        mauve: Color::Rgb(189, 147, 249),      // dracula purple
        red: Color::Rgb(255, 85, 85),          // dracula red
        maroon: Color::Rgb(230, 130, 130),
        peach: Color::Rgb(255, 184, 108),      // dracula orange
        yellow: Color::Rgb(241, 250, 140),     // dracula yellow
        green: Color::Rgb(80, 250, 123),       // dracula green
        teal: Color::Rgb(139, 233, 253),       // dracula cyan
        sky: Color::Rgb(98, 210, 228),         // slightly darker cyan
        sapphire: Color::Rgb(70, 180, 210),    // muted blue-cyan
        blue: Color::Rgb(125, 145, 200),        // brightened comment-blue for selections
        lavender: Color::Rgb(160, 140, 230),   // soft purple
        text: Color::Rgb(248, 248, 242),
        subtext1: Color::Rgb(222, 222, 212),
        subtext0: Color::Rgb(196, 196, 182),
        overlay2: Color::Rgb(150, 150, 160),
        overlay1: Color::Rgb(125, 125, 140),
        overlay0: Color::Rgb(98, 114, 164),    // dracula comment
        surface2: Color::Rgb(74, 82, 120),
        surface1: Color::Rgb(60, 67, 100),
        surface0: Color::Rgb(68, 71, 90),      // brighter for visible borders
        base: Color::Rgb(40, 42, 54),          // dracula bg
        mantle: Color::Rgb(33, 34, 44),
        crust: Color::Rgb(26, 27, 35),
    },
    // Gruvbox Dark
    Theme {
        rosewater: Color::Rgb(214, 153, 182),
        flamingo: Color::Rgb(211, 134, 155),
        pink: Color::Rgb(211, 134, 155),       // gruvbox bright purple
        mauve: Color::Rgb(177, 98, 134),       // gruvbox purple
        red: Color::Rgb(251, 73, 52),          // gruvbox bright red
        maroon: Color::Rgb(204, 36, 29),       // gruvbox red
        peach: Color::Rgb(254, 128, 25),       // gruvbox bright orange
        yellow: Color::Rgb(250, 189, 47),      // gruvbox bright yellow
        green: Color::Rgb(184, 187, 38),       // gruvbox bright green
        teal: Color::Rgb(142, 192, 124),       // gruvbox bright aqua
        sky: Color::Rgb(131, 165, 152),        // gruvbox aqua
        sapphire: Color::Rgb(69, 133, 136),    // gruvbox dark aqua
        blue: Color::Rgb(131, 165, 152),        // gruvbox bright_blue #83a598
        lavender: Color::Rgb(213, 196, 161),   // gruvbox fg2
        text: Color::Rgb(235, 219, 178),       // gruvbox fg1
        subtext1: Color::Rgb(213, 196, 161),   // gruvbox fg2
        subtext0: Color::Rgb(189, 174, 147),   // gruvbox fg3
        overlay2: Color::Rgb(168, 153, 132),   // gruvbox fg4
        overlay1: Color::Rgb(146, 131, 116),   // gruvbox gray
        overlay0: Color::Rgb(124, 111, 100),   // gruvbox bg4
        surface2: Color::Rgb(102, 92, 84),     // gruvbox bg3 #665c54
        surface1: Color::Rgb(80, 73, 69),      // gruvbox bg2 #504945
        surface0: Color::Rgb(60, 56, 54),      // gruvbox bg1 #3c3836
        base: Color::Rgb(40, 40, 40),          // gruvbox bg0
        mantle: Color::Rgb(29, 32, 33),        // gruvbox bg0_h
        crust: Color::Rgb(20, 20, 20),
    },
    // Nord
    Theme {
        rosewater: Color::Rgb(229, 233, 240),   // nord snow1
        flamingo: Color::Rgb(191, 97, 106),    // nord red (warm accent)
        pink: Color::Rgb(196, 155, 181),       // lightened nord15 purple
        mauve: Color::Rgb(180, 142, 173),      // nord15 purple
        red: Color::Rgb(191, 97, 106),         // nord11 red
        maroon: Color::Rgb(171, 77, 86),
        peach: Color::Rgb(208, 135, 112),      // nord12 orange
        yellow: Color::Rgb(235, 203, 139),     // nord13 yellow
        green: Color::Rgb(163, 190, 140),      // nord14 green
        teal: Color::Rgb(143, 188, 187),       // nord7 frost0
        sky: Color::Rgb(136, 192, 208),        // nord8 frost1
        sapphire: Color::Rgb(129, 161, 193),   // nord9 frost2
        blue: Color::Rgb(94, 129, 172),        // nord10 frost3
        lavender: Color::Rgb(143, 188, 187),   // nord7 frost0
        text: Color::Rgb(236, 239, 244),       // nord6 snow2
        subtext1: Color::Rgb(229, 233, 240),   // nord5 snow1
        subtext0: Color::Rgb(216, 222, 233),   // nord4 snow0
        overlay2: Color::Rgb(180, 186, 197),
        overlay1: Color::Rgb(144, 150, 161),
        overlay0: Color::Rgb(108, 114, 125),
        surface2: Color::Rgb(76, 86, 106),     // nord3 polar3
        surface1: Color::Rgb(67, 76, 94),      // nord2 polar2
        surface0: Color::Rgb(59, 66, 82),      // nord1 polar1
        base: Color::Rgb(46, 52, 64),          // nord polar0
        mantle: Color::Rgb(41, 46, 56),
        crust: Color::Rgb(36, 40, 48),
    },
];

fn t() -> &'static Theme {
    &THEMES[THEME_INDEX.load(Ordering::Relaxed)]
}

pub fn current_index() -> usize {
    THEME_INDEX.load(Ordering::Relaxed)
}

pub fn current_name() -> &'static str {
    THEME_NAMES[current_index()]
}

pub fn set_theme(index: usize) {
    THEME_INDEX.store(index % THEMES.len(), Ordering::Relaxed);
}

pub fn cycle_theme() {
    let next = (current_index() + 1) % THEMES.len();
    set_theme(next);
}

pub fn theme_count() -> usize {
    THEMES.len()
}

// Accessor functions — drop-in replacements for the old constants
pub fn rosewater() -> Color { t().rosewater }
pub fn flamingo() -> Color { t().flamingo }
pub fn pink() -> Color { t().pink }
pub fn mauve() -> Color { t().mauve }
pub fn red() -> Color { t().red }
pub fn maroon() -> Color { t().maroon }
pub fn peach() -> Color { t().peach }
pub fn yellow() -> Color { t().yellow }
pub fn green() -> Color { t().green }
pub fn teal() -> Color { t().teal }
pub fn sky() -> Color { t().sky }
pub fn sapphire() -> Color { t().sapphire }
pub fn blue() -> Color { t().blue }
pub fn lavender() -> Color { t().lavender }
pub fn text() -> Color { t().text }
pub fn subtext1() -> Color { t().subtext1 }
pub fn subtext0() -> Color { t().subtext0 }
pub fn overlay2() -> Color { t().overlay2 }
pub fn overlay1() -> Color { t().overlay1 }
pub fn overlay0() -> Color { t().overlay0 }
pub fn surface2() -> Color { t().surface2 }
pub fn surface1() -> Color { t().surface1 }
pub fn surface0() -> Color { t().surface0 }
pub fn base() -> Color { t().base }
pub fn mantle() -> Color { t().mantle }
pub fn crust() -> Color { t().crust }
