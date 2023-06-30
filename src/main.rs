use clap::Parser;
use std::time::{Duration, Instant};
use sysfs_gpio::{Direction, Pin};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    ready: u64,
    #[arg(short, long)]
    no_more: u64,
}

static COFFEE_READY: &'static str =  "https://hooks.slack.com/workflows/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";
static COFFEE_NO_MORE: &'static str = "https://hooks.slack.com/workflows/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";

enum Coffee {
    Ready,
    NoMore,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // coffee ready
    let ready = tokio::spawn(async move {
        let pin = Pin::new(args.ready);
        if let Err(e) = monitor_button(pin, Coffee::Ready).await {
            eprintln!("Error: {}", e);
        }
    });

    let no_more = tokio::spawn(async move {
        let pin = Pin::new(args.no_more);
        if let Err(e) = monitor_button(pin, Coffee::NoMore).await {
            eprintln!("Error: {}", e);
        }
    });

    let _ = (ready.await, no_more.await);
}

async fn monitor_button(pin: Pin, program: Coffee) -> sysfs_gpio::Result<()> {
    pin.export()?;
    pin.set_direction(Direction::In)?;
    pin.set_edge(sysfs_gpio::Edge::BothEdges)?;
    let mut poller = pin.get_poller()?;
    let mut now = Instant::now() - Duration::from_secs(60);
    loop {
        match poller.poll(10)? {
            Some(value) => {
                if value == 1 {
                    let elapsed = Instant::now() - now;
                    if elapsed.as_secs() < 60 {
                        continue;
                    }

                    match program {
                        Coffee::Ready => send_slack_message(COFFEE_READY).await,
                        Coffee::NoMore => send_slack_message(COFFEE_NO_MORE).await,
                    }

                    now = Instant::now();
                }
            }
            None => (),
        }
    }
}

async fn send_slack_message(url: &str) -> () {
    println!("Sending slack message: {}", url);
    match reqwest::get(url).await {
        Ok(v) => {
            println!("Response: {}", v.status());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
