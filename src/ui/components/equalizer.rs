use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, Widget},
};

use crate::app::state::EqStyle;
use crate::audio::types::SpectrumData;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, spectrum: &SpectrumData, peaks: &[f32], style: EqStyle) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::surface1()))
        .style(Style::default().bg(theme::base()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let eq = EqWidget { spectrum, peaks, style };
    f.render_widget(eq, inner);
}

struct EqWidget<'a> {
    spectrum: &'a SpectrumData,
    peaks: &'a [f32],
    style: EqStyle,
}

// Bottom-up fractional block characters (1/8 to 8/8)
const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

impl Widget for EqWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        match self.style {
            EqStyle::Bars => render_bars(self.spectrum, area, buf),
            EqStyle::Blocks => render_blocks(self.spectrum, area, buf),
            EqStyle::Peaks => render_peaks(self.spectrum, self.peaks, area, buf),
            EqStyle::Mirror => render_mirror(self.spectrum, area, buf),
            EqStyle::Wave => render_wave(self.spectrum, area, buf),
        }
    }
}

/// Calculate bar layout to fill the full width.
/// Returns (num_bars, bar_width, gap, step, x_offset).
fn bar_layout(width: u16) -> (u16, u16, u16, u16, u16) {
    let bar_width = 2u16;
    let gap = 1u16;
    let step = bar_width + gap;
    let num_bars = (width + gap) / step;
    let num_bars = num_bars.max(1);
    let used = num_bars * step - gap;
    let x_offset = (width.saturating_sub(used)) / 2;
    (num_bars, bar_width, gap, step, x_offset)
}

/// Interpolate spectrum bins to the desired number of bars.
fn interpolate_bins(bins: &[f32], num_bars: u16) -> Vec<f32> {
    let n = num_bars as usize;
    let src_len = bins.len();
    if src_len == 0 {
        return vec![0.0; n];
    }
    (0..n)
        .map(|i| {
            let pos = i as f32 * (src_len - 1) as f32 / (n - 1).max(1) as f32;
            let lo = (pos as usize).min(src_len - 1);
            let hi = (lo + 1).min(src_len - 1);
            let frac = pos - lo as f32;
            bins[lo] * (1.0 - frac) + bins[hi] * frac
        })
        .collect()
}

// === Style 1: Classic solid bars ===

fn render_bars(spectrum: &SpectrumData, area: Rect, buf: &mut Buffer) {
    let (num_bars, bar_width, _gap, step, x_off) = bar_layout(area.width);
    let vals = interpolate_bins(&spectrum.bins, num_bars);

    for (bar_idx, &val) in vals.iter().enumerate() {
        let max_eighths = area.height as f32 * 8.0;
        let filled_eighths = (val * max_eighths) as usize;
        let full_rows = filled_eighths / 8;
        let remainder = filled_eighths % 8;
        let color = gradient_color(val);
        let x_start = area.x + x_off + bar_idx as u16 * step;

        for row in 0..area.height {
            let y = area.y + area.height - 1 - row;
            let row_idx = row as usize;
            for dx in 0..bar_width {
                let x = x_start + dx;
                if x >= area.x + area.width { break; }
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

// === Style 2: Winamp-style discrete blocks ===

fn render_blocks(spectrum: &SpectrumData, area: Rect, buf: &mut Buffer) {
    let (num_bars, bar_width, _gap, step, x_off) = bar_layout(area.width);
    let vals = interpolate_bins(&spectrum.bins, num_bars);
    let block_height = 2u16;

    for (bar_idx, &val) in vals.iter().enumerate() {
        let total_blocks = (area.height / block_height) as f32;
        let filled = (val * total_blocks).ceil() as u16;
        let x_start = area.x + x_off + bar_idx as u16 * step;

        for block_idx in 0..filled {
            let block_bottom = area.y + area.height - 1 - block_idx * block_height;
            let block_top = block_bottom.saturating_sub(block_height - 1);
            let intensity = block_idx as f32 / total_blocks;
            let color = gradient_color(intensity);

            for y in block_top..=block_bottom.min(area.y + area.height - 1) {
                if y == block_top && block_idx > 0 { continue; }
                for dx in 0..bar_width {
                    let x = x_start + dx;
                    if x >= area.x + area.width { break; }
                    let cell = &mut buf[(x, y)];
                    cell.set_char('█');
                    cell.set_fg(color);
                }
            }
        }
    }
}

// === Style 3: Bars with floating peak dots ===

fn render_peaks(spectrum: &SpectrumData, peaks: &[f32], area: Rect, buf: &mut Buffer) {
    let (num_bars, bar_width, _gap, step, x_off) = bar_layout(area.width);
    let vals = interpolate_bins(&spectrum.bins, num_bars);
    let peak_vals = interpolate_bins(peaks, num_bars);

    for (bar_idx, &val) in vals.iter().enumerate() {
        let max_eighths = area.height as f32 * 8.0;
        let filled_eighths = (val * max_eighths) as usize;
        let full_rows = filled_eighths / 8;
        let remainder = filled_eighths % 8;
        let color = gradient_color(val);
        let x_start = area.x + x_off + bar_idx as u16 * step;

        // Draw the bar
        for row in 0..area.height {
            let y = area.y + area.height - 1 - row;
            let row_idx = row as usize;
            for dx in 0..bar_width {
                let x = x_start + dx;
                if x >= area.x + area.width { break; }
                let cell = &mut buf[(x, y)];
                if row_idx < full_rows {
                    cell.set_char('▓');
                    cell.set_fg(color);
                } else if row_idx == full_rows && remainder > 0 {
                    cell.set_char(BLOCKS[(remainder - 1).min(7)]);
                    cell.set_fg(color);
                }
            }
        }

        // Draw the floating peak indicator
        let peak_val = peak_vals[bar_idx];
        let peak_row = (peak_val * area.height as f32) as u16;
        if peak_row > 0 && peak_row <= area.height {
            let y = area.y + area.height - peak_row;
            let peak_color = theme::text();
            for dx in 0..bar_width {
                let x = x_start + dx;
                if x >= area.x + area.width { break; }
                let cell = &mut buf[(x, y)];
                cell.set_char('▔');
                cell.set_fg(peak_color);
            }
        }
    }
}

// === Style 4: Mirror (symmetric from center) ===

fn render_mirror(spectrum: &SpectrumData, area: Rect, buf: &mut Buffer) {
    let (num_bars, bar_width, _gap, step, x_off) = bar_layout(area.width);
    let vals = interpolate_bins(&spectrum.bins, num_bars);
    let mid = area.y + area.height / 2;
    let half_h = area.height / 2;

    for (bar_idx, &val) in vals.iter().enumerate() {
        let max_eighths = half_h as f32 * 8.0;
        let filled_eighths = (val * max_eighths) as usize;
        let full_rows = filled_eighths / 8;
        let remainder = filled_eighths % 8;
        let color = gradient_color(val);
        let x_start = area.x + x_off + bar_idx as u16 * step;

        // Top half (grows upward from center)
        for row in 0..half_h {
            let y = mid.saturating_sub(1 + row);
            if y < area.y { break; }
            let row_idx = row as usize;
            for dx in 0..bar_width {
                let x = x_start + dx;
                if x >= area.x + area.width { break; }
                let cell = &mut buf[(x, y)];
                if row_idx < full_rows {
                    cell.set_char('█');
                    cell.set_fg(color);
                } else if row_idx == full_rows && remainder > 0 {
                    cell.set_char(BLOCKS[7 - (remainder - 1)]);
                    cell.set_fg(color);
                }
            }
        }

        // Bottom half (grows downward from center)
        // Uses fg/bg swap trick: fractional block chars fill from bottom in fg,
        // so we set fg=background and bg=bar color to get top-down fill visually.
        for row in 0..half_h {
            let y = mid + row;
            if y >= area.y + area.height { break; }
            let row_idx = row as usize;
            for dx in 0..bar_width {
                let x = x_start + dx;
                if x >= area.x + area.width { break; }
                let cell = &mut buf[(x, y)];
                if row_idx < full_rows {
                    cell.set_char('█');
                    cell.set_fg(color);
                } else if row_idx == full_rows && remainder > 0 {
                    cell.set_char(BLOCKS[7 - remainder]);
                    cell.set_fg(theme::base());
                    cell.set_bg(color);
                }
            }
        }
    }
}

// === Style 5: Wave / line graph ===

fn render_wave(spectrum: &SpectrumData, area: Rect, buf: &mut Buffer) {
    // Interpolate to every column for smooth wave
    let vals = interpolate_bins(&spectrum.bins, area.width);

    for (col, &val) in vals.iter().enumerate() {
        let x = area.x + col as u16;
        if x >= area.x + area.width { break; }

        let bar_height = (val * area.height as f32) as u16;

        // Draw the wave line at the top
        if bar_height > 0 {
            let wave_y = area.y + area.height - bar_height;
            if wave_y >= area.y && wave_y < area.y + area.height {
                let cell = &mut buf[(x, wave_y)];
                cell.set_char('━');
                cell.set_fg(theme::text());
            }
        }

        // Fill underneath with dimmer color
        for row in 0..bar_height.saturating_sub(1) {
            let y = area.y + area.height - 1 - row;
            if y <= area.y { break; }
            let fill_intensity = row as f32 / area.height as f32;
            let fill_color = gradient_color(fill_intensity * 0.6);
            let cell = &mut buf[(x, y)];
            cell.set_char('░');
            cell.set_fg(fill_color);
        }
    }
}

fn gradient_color(val: f32) -> ratatui::style::Color {
    if val < 0.2 {
        theme::green()
    } else if val < 0.4 {
        theme::teal()
    } else if val < 0.6 {
        theme::yellow()
    } else if val < 0.8 {
        theme::peach()
    } else {
        theme::red()
    }
}
