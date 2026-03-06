use ratatui::Frame;

use crate::ui::theme;
use super::popup::draw_selector;

pub fn draw(f: &mut Frame) {
    draw_selector(f, " Theme ", theme::THEME_NAMES, theme::current_index());
}
