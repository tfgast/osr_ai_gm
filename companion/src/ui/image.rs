use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui_image::StatefulImage;

use crate::app::App;

pub fn render_image(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title(" Image ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if !app.show_image {
        let p = Paragraph::new("(hidden — press i to show)")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(p, inner);
        return;
    }

    match app.image_state.as_mut() {
        Some(protocol) => {
            let image_widget = StatefulImage::new(None);
            f.render_stateful_widget(image_widget, inner, protocol);
        }
        None => {
            let p = Paragraph::new("(no image)")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            f.render_widget(p, inner);
        }
    }
}
