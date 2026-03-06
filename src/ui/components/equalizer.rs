use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, Widget},
};

use crate::audio::types::SpectrumData;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, spectrum: &SpectrumData) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::SURFACE0))
        .style(Style::default().bg(theme::BASE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let eq = BarEq { spectrum };
    f.render_widget(eq, inner);
}

/// Renders spectrum as solid block columns using fractional block characters
struct BarEq<'a> {
    spectrum: &'a SpectrumData,
}

// Bottom-up fractional block characters (1/8 to 8/8)
const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

impl Widget for BarEq<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let num_bins = self.spectrum.bins.len();
        let bar_width = 2u16;
        let gap = 1u16;
        let step = bar_width + gap;
        let max_bars = (area.width + gap) / step;

        for bar_idx in 0..max_bars.min(num_bins as u16) {
            // Map bar to a spectrum bin
            let bin_idx = (bar_idx as usize * num_bins) / max_bars as usize;
            let bin_idx = bin_idx.min(num_bins - 1);
            let val = self.spectrum.bins[bin_idx];

            let max_eighths = area.height as f32 * 8.0;
            let filled_eighths = (val * max_eighths) as usize;

            let full_rows = filled_eighths / 8;
            let remainder = filled_eighths % 8;

            let color = gradient_color(val);
            let x_start = area.x + bar_idx * step;

            // Draw from bottom up
            for row in 0..area.height {
                let y = area.y + area.height - 1 - row;
                let row_idx = row as usize;

                for dx in 0..bar_width {
                    let x = x_start + dx;
                    if x >= area.x + area.width {
                        break;
                    }

                    let cell = &mut buf[(x, y)];

                    if row_idx < full_rows {
                        cell.set_char('█');
                        cell.set_fg(color);
                    } else if row_idx == full_rows && remainder > 0 {
                        cell.set_char(BLOCKS[remainder - 1]);
                        cell.set_fg(color);
                    }
                }
            }
        }
    }
}

fn gradient_color(val: f32) -> ratatui::style::Color {
    if val < 0.2 {
        theme::GREEN
    } else if val < 0.4 {
        theme::TEAL
    } else if val < 0.6 {
        theme::YELLOW
    } else if val < 0.8 {
        theme::PEACH
    } else {
        theme::RED
    }
}
