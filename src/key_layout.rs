use termion::event::Key;
pub struct KeyLayout {
    pub up: Key,
    pub down: Key,
    pub left: Key,
    pub right: Key,
    pub up_arrow: Key,
    pub down_arrow: Key,
    pub left_arrow: Key,
    pub right_arrow: Key,
    pub right_alt: Key,
    pub escape: Key,
    pub quit: Key,
    pub search: Key,
    pub create: Key,
}

impl Default for KeyLayout {
    fn default() -> Self {
        KeyLayout {
            up: Key::Char('k'),
            up_arrow: Key::Up,
            down: Key::Char('j'),
            down_arrow: Key::Down,
            left: Key::Char('h'),
            left_arrow: Key::Right,
            right: Key::Char('l'),
            right_alt: Key::Char('\n'),
            right_arrow: Key::Right,
            escape: Key::Esc,
            quit: Key::Char('q'),
            search: Key::Char('s'),
            create: Key::Char('c'),
        }
    }
}
