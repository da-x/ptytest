mod tty;
mod commands;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::time::{Instant, Duration};
use std::io::{Write, Read};
use std::io::ErrorKind;

use mio::{Poll, PollOpt, Ready, Events};
#[cfg(not(windows))]
use mio::unix::UnixReady;

use tty::{EventedReadWrite, EventedPty, Pty};
use vt100;

pub use term::SizeInfo;
pub use commands::{Code, AsAnsi};

pub struct PtyTest {
    pty: Pty,
    wait_timeout: Duration,
    poll: Poll,
    events: Option<Events>,
    buf: [u8; 0x1000],
    parser: vt100::Parser,
}

#[derive(Debug)]
pub enum Error {
    ProcessExited(ScreenDiff, AsciiScreen),
    IoError(std::io::Error),
    TimeoutForScreenState(ScreenDiff, AsciiScreen),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "PTYError {{")?;

        let maybe_ascii_state = match self {
            Error::TimeoutForScreenState(screen_diff, ascii_state) => {
                for line in format!("{}", screen_diff).split("\n") {
                    writeln!(f, "  {}", line)?;
                }

                Some(ascii_state)
            }
            Error::ProcessExited(screen_diff, ascii_state) => {
                for line in format!("{}", screen_diff).split("\n") {
                    writeln!(f, "  {}", line)?;
                }

                Some(ascii_state)
            }
            Error::IoError(_) => {
                None
            }
        };

        if let Some(ascii_state) = maybe_ascii_state {
            writeln!(f, "  Screen {{")?;
            for line in ascii_state.as_fragments().split("\n") {
                if !line.is_empty() {
                    writeln!(f, "    {}", line)?;
                }
            }
            writeln!(f, "  }}")?;
        }

        match self {
            Error::TimeoutForScreenState{..} => {
                writeln!(f, "  Screen timeout")?;
            }
            Error::ProcessExited{..} => {
                writeln!(f, "  Process exited")?;
            }
            Error::IoError(err) => {
                writeln!(f, "  IoError: {:?}", err)?;
            }
        };

        writeln!(f, "}}")?;
        Ok(())
    }
}

#[macro_export]
macro_rules! ascii_screen {
    ($($x:tt)*) => {AsciiScreen::new(file!(), line!(), &[
        $(ascii_screen_fragment!{$x}),*
    ])};
}

pub enum AsciiScreenFragment {
    Newline,
    String(&'static str),
    Underscore(usize),
    CursorPosition,
    Nothing,
}

impl AsciiScreenFragment {
    pub fn by_ident(s: &'static str) -> Self {
        let mut underscore_prefix = 0;

        for i in s.chars() {
            if i == '_' {
                underscore_prefix += 1;
                continue;
            } else {
                panic!("unknown character in indent {:?}", i);
            }
        }

        return AsciiScreenFragment::Underscore(underscore_prefix);
    }
}

#[derive(Debug)]
pub struct AsciiScreen {
    source_info: Option<(&'static str, u32)>,
    contents: String,
    cursor_rowcol: Option<(u16, u16)>,
}

impl AsciiScreen {
    pub fn new(file: &'static str, line: u32, asf_list: &[AsciiScreenFragment]) -> Self {
        let mut contents = String::new();
        let mut rows = 0;
        let mut cols = 0;
        let mut cursor_rowcol = None;

        for asf in asf_list {
            match asf {
                AsciiScreenFragment::String(line) => {
                    cols = 0;
                    contents.push_str(line);
                }
                AsciiScreenFragment::Newline => {
                    contents.push_str("\n");
                    rows += 1;
                }
                AsciiScreenFragment::Underscore(added_cols)  => {
                    cols += *added_cols;
                }
                AsciiScreenFragment::CursorPosition => {
                    if cursor_rowcol.is_some() {
                        panic!("more than one cursor defined"); // TODO;
                    }
                    cursor_rowcol = Some((rows as u16, cols as u16));
                }
                AsciiScreenFragment::Nothing => {
                    continue
                }
            }
        }

        if contents.ends_with("\n") {
            contents.pop();
        }

        AsciiScreen {
            source_info: Some((file, line)),
            contents,
            cursor_rowcol,
        }
    }

    fn as_fragments(&self) -> String {
        use std::fmt::Write;

        let mut s = String::new();

        for (_idx, line) in self.contents.split("\n").enumerate() {
            let _ = writeln!(&mut s, "{:?}, NL, ", line);

            //
            // TODO: write cursor information
            //
            // if let Some((row, col)) = self.cursor_rowcol {
            //     if row == idx as u16 + 1 {
            //         println!(",____________^,");
            //     }
            // }
        }

        s
    }
}

#[derive(Debug)]
pub struct Change<E> {
    expected: E,
    received: E,
}

#[derive(Debug)]
pub struct ScreenDiff {
    cursor_pos_diff: Option<Change<Option<(u16, u16)>>>,
    content_diff: Option<(String, String)>,
}

impl std::fmt::Display for ScreenDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ScreenDiff {{")?;

        match &self.content_diff {
            None => {},
            Some((expected, found)) => {
                let changeset = difference::Changeset::new(&expected, &found, "\n");
                use difference::Difference;

                for change in changeset.diffs {
                    match change {
                        Difference::Rem(x) => {
                            for line in x.split("\n") {
                                writeln!(f, "  -{}", line)?;
                            }
                        },
                        Difference::Add(x) => {
                            for line in x.split("\n") {
                                writeln!(f, "  +{}", line)?;
                            }
                        },
                        Difference::Same(x) => {
                            for line in x.split("\n") {
                                writeln!(f, "   {}", line)?;
                            }
                        },
                    }
                }
            }
        }

        write!(f, "}}")?;
        Ok(())
    }
}

#[macro_export]
macro_rules! ascii_screen_fragment {
    ($x:literal) => { AsciiScreenFragment::String($x) };
    (NL) => { AsciiScreenFragment::Newline };
    ($x:ident) => { AsciiScreenFragment::by_ident(stringify!($x)) };
    (^) => { AsciiScreenFragment::CursorPosition };
    (,) => { AsciiScreenFragment::Nothing };
}

impl PtyTest {
    fn from_tty(mut pty: Pty, size: &SizeInfo) -> Self {
        let poll = mio::Poll::new().expect("create mio Poll");
        let mut tokens = (0..).map(Into::into);
        let poll_opts = PollOpt::edge();
        let parser = vt100::Parser::new(size.lines as u16, size.cols as u16, 0);

        pty.register(&poll, &mut tokens, Ready::readable(), poll_opts).unwrap();

        Self {
            pty,
            parser,
            buf: [0u8; 0x1000],
            events: Some(Events::with_capacity(128)),
            wait_timeout: Duration::from_millis(1000),
            poll,
        }
    }

    pub fn ascii_state(&self) -> AsciiScreen {
        let screen = self.parser.screen();

        let cursor = if screen.hide_cursor() {
            None
        } else {
            Some(screen.cursor_position())
        };

        AsciiScreen {
            source_info: None,
            contents: screen.contents(),
            cursor_rowcol: cursor,
        }
    }

    pub fn diff(&self, screen_state: &AsciiScreen) -> Result<(), ScreenDiff> {
        let screen = self.parser.screen();
        let mut content_diff = None;
        let mut cursor_pos_diff = None;

        if screen_state.contents != screen.contents() {
            content_diff = Some((screen_state.contents.clone(), screen.contents()));
        }

        let cursor = if screen.hide_cursor() {
            None
        } else {
            Some(screen.cursor_position())
        };

        if cursor != screen_state.cursor_rowcol {
            cursor_pos_diff = Some(Change {
                expected: screen_state.cursor_rowcol,
                received: cursor,
            });
        }

        if cursor_pos_diff.is_some() || content_diff.is_some() {
            return Err(ScreenDiff {
                cursor_pos_diff,
                content_diff,
            });
        }

        Ok(())
    }

    pub fn write_str(&mut self, codes: &str) -> Result<(), Error> {
        let writer = self.pty.writer();
        match writer.write(codes.as_bytes()) {
            Err(io) => return Err(Error::IoError(io)),
            Ok(_n) => if _n != codes.len() { unimplemented!() },
        }

        Ok(())
    }

    pub fn write<D, T>(&mut self, code: D) -> Result<(), Error>
        where D: AsRef<T>, T: commands::AsAnsi
    {
        let mut s = String::new();
        code.as_ref().add_to_string(&mut s);
        self.write_str(&s)
    }

    pub fn wait_for(&mut self, screen_state: &AsciiScreen) -> Result<(), Error> {
        let mut events = self.events.take().unwrap();
        let end_time = Instant::now() + self.wait_timeout;
        let mut exited = false;
        let mut had_data = false;

        loop {
            let diff = match self.diff(screen_state) {
                Ok(()) => {
                    self.events = Some(events);
                    return Ok(());
                },
                Err(diff) => {
                    if Instant::now() >= end_time {
                        return Err(Error::TimeoutForScreenState(diff, self.ascii_state()))
                    }

                    diff
                }
            };

            if !had_data && exited {
                // println!("{:?} {:?} != {:?}", self.parser.screen().contents(), self.parser.screen().cursor_position(),
                //     screen_state);
                self.events = Some(events);
                return Err(Error::ProcessExited(diff, self.ascii_state()));
            }

            if let Err(err) = self.poll.poll(&mut events, Some(self.wait_timeout / 10)) {
                match err.kind() {
                    ErrorKind::Interrupted => continue,
                    _ => panic!("EventLoop polling error: {:?}", err),
                }
            }

            had_data = false;
            for event in events.iter() {
                match event.token() {
                    token if token == self.pty.child_event_token() => {
                        if let Some(tty::ChildEvent::Exited) = self.pty.next_child_event() {
                            exited = true;
                        }
                    }
                    token if token == self.pty.read_token() => {
                        had_data = true;
                        #[cfg(unix)]
                        {
                            if UnixReady::from(event.readiness()).is_hup() {
                                // don't try to do I/O on a dead PTY
                                continue;
                            }
                        }

                        if event.readiness().is_readable() {
                            match self.pty.reader().read(&mut self.buf[..]) {
                                Ok(nread) => {
                                    self.parser.process(&self.buf[0..nread]);
                                }
                                Err(err) => {
                                    self.events = Some(events);
                                    return Err(Error::IoError(err));
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    pub fn new_with_args<S>(program: S, args: Vec<String>, size: &SizeInfo) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        let tty = pty_new_with_args(program, args, size);
        Self::from_tty(tty, size)
    }
}

fn pty_new_with_args<S>(program: S, args: Vec<String>, size: &SizeInfo) -> tty::Pty
where
    S: Into<Cow<'static, str>>,
{
    let config = config::Config::new().set_shell(config::Shell::new_with_args(program, args));
    let tty = tty::new(&config, size);

    tty
}

impl config::Config<()> {
    fn new() -> Self {
        Self {
            c: PhantomData,
            shell: None,
            env: BTreeMap::new(),
        }
    }

    fn set_shell(self, shell: config::Shell<'static>) -> Self {
        Self {
            shell: Some(shell),
            ..self
        }
    }
}

impl SizeInfo {
    pub fn new(cols: usize, lines: usize) -> Self {
        Self {
            lines, cols, width: 10, height: 5,
        }
    }
}

// Alacritty-based stubs for used by `tty`
////////////////////////////////////////////////////////////////////////////////

pub mod unused {
    pub use crate::tty::{process_should_exit, child_pid, setup_env};
}

mod event {
    use crate::SizeInfo;

    pub trait OnResize {
        fn on_resize(&mut self, size: &SizeInfo);
    }
}

mod term {
    pub struct SizeInfo {
        pub(crate) lines: usize,
        pub(crate) cols: usize,
        pub width: usize,
        pub height: usize,
    }

    impl SizeInfo {
        pub fn lines(&self) -> (usize, ()) {
            (self.lines, ())
        }

        pub fn cols(&self) -> (usize, ()) {
            (self.cols, ())
        }
    }
}

mod config {
    use std::collections::BTreeMap;
    use std::borrow::Cow;
    use std::path::PathBuf;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Shell<'a> {
        pub program: Cow<'a, str>,
        pub args: Vec<String>,
    }

    impl<'a> Shell<'a> {
        pub fn new<S>(program: S) -> Shell<'a>
        where
            S: Into<Cow<'a, str>>,
        {
            Shell { program: program.into(), args: Vec::new() }
        }

        pub fn new_with_args<S>(program: S, args: Vec<String>) -> Shell<'a>
        where
            S: Into<Cow<'a, str>>,
        {
            Shell { program: program.into(), args }
        }
    }

    pub struct Config<C> {
        pub(crate) c: std::marker::PhantomData<C>,
        pub shell: Option<Shell<'static>>,
        pub env: BTreeMap<String, String>,
    }

    impl<C> Config<C> {
        pub fn working_directory(&self) -> Option<PathBuf> {
            None
        }
    }
}
