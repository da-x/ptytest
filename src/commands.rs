pub trait AsAnsi {
    fn add_to_string(&self, out: &mut String);
}

pub enum Code {
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    End,
    Home,
}

impl AsRef<Code> for Code {
    fn as_ref(&self) -> &Code {
        self
    }
}

impl<T> AsAnsi for T
    where T: AsRef<str>
{
    fn add_to_string(&self, out: &mut String) {
        out.push_str(self.as_ref());
    }
}

impl AsAnsi for Code {
    fn add_to_string(&self, out: &mut String) {
        let s = match *self {
            Code::Left => "\x1b[D",
            Code::Right => "\x1b[C",
            Code::Up => "\x1b[A",
            Code::Down => "\x1b[B",
            Code::PageUp => "\x1b[5~",
            Code::PageDown => "\x1b[6~",
            Code::Home => "\x1b[1~",
            Code::End => "\x1b[4~",
        };

        *out += s;
    }
}
