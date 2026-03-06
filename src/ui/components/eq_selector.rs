use ratatui::Frame;

use crate::app::state::EqStyle;
use super::popup::draw_selector;

pub fn draw(f: &mut Frame, current: EqStyle) {
    let labels: Vec<&str> = EqStyle::ALL.iter().map(|s| s.label()).collect();
    let selected = EqStyle::ALL.iter().position(|&s| s == current).unwrap_or(0);
    draw_selector(f, " Visualizer ", &labels, selected);
}
