use ptytest::{SizeInfo, PtyTest, Code};
use ptytest::{ascii_screen, ascii_screen_fragment, AsciiScreen, AsciiScreenFragment};

fn main() -> Result<(), ptytest::Error> {
    let mut args = std::env::args();
    let _current_exe = args.next();
    let check_me = args.next().expect("executable path of check_me");

    println!("Checking executable program {:?}", check_me);

    let mut pty_test = PtyTest::new_with_args(check_me, vec![], &SizeInfo::new(30, 15));
    let _ = pty_test.wait_for(&ascii_screen!{
        "Hello world.", NL,
        "Please enter a command:", NL,
        "Prompt >> ",
        ,__________^,
    })?;

    pty_test.write_str("bla")?;
    let _ = pty_test.wait_for(&ascii_screen!{
        "Hello world.", NL,
        "Please enter a command:", NL,
        "Prompt >> bla",
        ,_____________^,
    })?;

    pty_test.write(&Code::Left)?;
    pty_test.write(&Code::Left)?;
    let _ = pty_test.wait_for(&ascii_screen!{
        "Hello world.", NL,
        "Please enter a command:", NL,
        "Prompt >> bla",
        ,___________^,
    })?;

    pty_test.write_str("x")?;
    let _ = pty_test.wait_for(&ascii_screen!{
        "Hello world.", NL,
        "Please enter a command:", NL,
        "Prompt >> bxla",
        ,____________^,
    })?;

    println!("Passed all checks.");

    Ok(())
}
