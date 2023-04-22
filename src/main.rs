use crate::args::{Args, Parser};
use std::thread::sleep;
use std::time::{Duration, Instant};

mod args;

fn main() {
    let args = Args::parse();

    match args.command {
        args::Command::Local {
            work,
            short,
            long,
        } => println!("work: {work}, short: {short}, long: {long}"),
        _ => todo!(),
    }
}

/* fn local_loop(work: Duration, short_break: Duration, long_break: Duration) {
    // press start

    let now = Instant::now();
    loop {
        let elapsed = now.elapsed();
        // clear console
        println!("Elapsed: {} seconds", elapsed.as_secs());

        if elapsed >= work {
            break;
        }

        sleep(Duration::from_secs(1));
    }

    println!("Stopping.");
} */
