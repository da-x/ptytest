use rustyline::error::ReadlineError;
use rustyline::Editor;

fn main() {
    println!("Hello world.");
    println!("Please enter a command:");

    let mut rl = Editor::<()>::new();
    loop {
        let readline = rl.readline("Prompt >> ");
        match readline {
            Ok(line) => {
                if line == "exit" {
                    break
                }
                println!("Line: {}", line);
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }
}
