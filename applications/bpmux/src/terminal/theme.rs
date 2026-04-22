use ratatui::style::{Color, Modifier, Style};

pub(super) const DRACULA_BG: Color = Color::Rgb(40, 42, 54);
pub(super) const DRACULA_FG: Color = Color::Rgb(248, 248, 242);
pub(super) const DRACULA_SELECTION: Color = Color::Rgb(68, 71, 90);
pub(super) const DRACULA_COMMENT: Color = Color::Rgb(136, 148, 184);
pub(super) const DRACULA_PURPLE: Color = Color::Rgb(189, 147, 249);
pub(crate) const DRACULA_CYAN: Color = Color::Rgb(139, 233, 253);
pub(super) const DRACULA_ORANGE: Color = Color::Rgb(255, 184, 108);
pub(super) const DRACULA_GREEN: Color = Color::Rgb(80, 250, 123);
pub(super) const DRACULA_RED: Color = Color::Rgb(255, 85, 85);
pub(super) const DRACULA_PINK: Color = Color::Rgb(255, 121, 198);
pub(super) const DRACULA_YELLOW: Color = Color::Rgb(241, 250, 140);

pub(super) const SENDER_PALETTE: [Color; 6] = [
    DRACULA_CYAN,   // Dracula cyan
    DRACULA_GREEN,  // Dracula green
    DRACULA_ORANGE, // Dracula orange
    DRACULA_PURPLE, // Dracula purple
    DRACULA_PINK,   // Dracula pink
    DRACULA_YELLOW, // Dracula yellow
];

/// Dracula-themed stylesheet for tui-markdown rendering.
/// Replaces default ANSI colors that have poor contrast on dark backgrounds.
#[derive(Clone, Debug)]
pub(super) struct DraculaStyleSheet;

impl tui_markdown::StyleSheet for DraculaStyleSheet {
    fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::new()
                .fg(DRACULA_BG)
                .bg(DRACULA_CYAN)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            2 => Style::new().fg(DRACULA_CYAN).add_modifier(Modifier::BOLD),
            3 => Style::new()
                .fg(DRACULA_CYAN)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            _ => Style::new().fg(DRACULA_CYAN).add_modifier(Modifier::ITALIC),
        }
    }

    fn code(&self) -> Style {
        Style::new().fg(DRACULA_FG).bg(DRACULA_SELECTION)
    }

    fn link(&self) -> Style {
        Style::new()
            .fg(DRACULA_PURPLE)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn blockquote(&self) -> Style {
        Style::new().fg(DRACULA_GREEN)
    }

    fn heading_meta(&self) -> Style {
        Style::new().add_modifier(Modifier::DIM)
    }

    fn metadata_block(&self) -> Style {
        Style::new().fg(DRACULA_YELLOW) // Dracula yellow
    }
}

fn sender_color_index(name: &str) -> usize {
    let hash: usize = name.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as usize)
    });
    hash % SENDER_PALETTE.len()
}

pub(crate) fn sender_color(name: &str) -> Color {
    SENDER_PALETTE[sender_color_index(name)]
}

/// Color for channel tags. Broadcast = subdued Dracula comment gray.
/// Directed channels = sender palette offset by 3 so channel and sender
/// colors don't collide for the same name.
pub(super) fn channel_tag_color(tag: &str, is_broadcast: bool) -> Color {
    if is_broadcast {
        DRACULA_COMMENT // Dracula comment gray (#6272A4)
    } else {
        SENDER_PALETTE[(sender_color_index(tag) + 3) % SENDER_PALETTE.len()]
    }
}
