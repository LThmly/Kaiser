use ratatui_image::{
    FontSize,
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
};
/// Detect graphics without starting a competing reader on terminal input.
/// A text-only terminal gets the normal title, never a character-art substitute.
#[allow(deprecated)] // Explicit font size avoids the picker's blocking stdin query thread.
pub fn detect() -> Option<StatefulProtocol> {
    let term = std::env::var("TERM").unwrap_or_default();
    let program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let protocol = terminal_protocol(&term, &program)?;
    let font = crossterm::terminal::window_size()
        .ok()
        .and_then(|size| {
            let width = size.width.checked_div(size.columns)?;
            let height = size.height.checked_div(size.rows)?;
            (width > 0 && height > 0).then_some(FontSize::new(width, height))
        })
        .unwrap_or(FontSize::new(8, 16));
    let mut picker = Picker::from_fontsize(font);
    picker.set_protocol_type(protocol);
    from_picker(&picker)
}

fn terminal_protocol(term: &str, program: &str) -> Option<ProtocolType> {
    let term = term.to_ascii_lowercase();
    let program = program.to_ascii_lowercase();
    if term.contains("kitty") || term.contains("ghostty") || program == "ghostty" {
        Some(ProtocolType::Kitty)
    } else if program.contains("iterm") || program == "wezterm" {
        Some(ProtocolType::Iterm2)
    } else if term.contains("sixel") || term == "foot" || term == "foot-extra" {
        Some(ProtocolType::Sixel)
    } else {
        None
    }
}

fn from_picker(picker: &Picker) -> Option<StatefulProtocol> {
    if picker.protocol_type() == ProtocolType::Halfblocks {
        return None;
    }
    let image = image::load_from_memory(include_bytes!("../assets/kaiser.png")).ok()?;
    Some(picker.new_resize_protocol(image))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_png_is_used_only_with_real_image_protocols() {
        assert!(terminal_protocol("xterm-256color", "").is_none());
        assert!(terminal_protocol("dumb", "").is_none());
        assert_eq!(
            terminal_protocol("xterm-kitty", ""),
            Some(ProtocolType::Kitty)
        );
        assert_eq!(
            terminal_protocol("xterm-256color", "WezTerm"),
            Some(ProtocolType::Iterm2)
        );
        let mut picker = Picker::halfblocks();
        assert!(from_picker(&picker).is_none());
        for protocol in [
            ProtocolType::Kitty,
            ProtocolType::Iterm2,
            ProtocolType::Sixel,
        ] {
            picker.set_protocol_type(protocol);
            assert!(from_picker(&picker).is_some());
        }
        let image = image::load_from_memory(include_bytes!("../assets/kaiser.png")).unwrap();
        assert_eq!((image.width(), image.height()), (1024, 1024));
    }
}
