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
    #[arg(short, long, env)]
    ready_url: String,
    #[arg(short, long, env)]
    no_more_url: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let coffee_ready_url = args.ready_url.clone();
    let coffee_no_more_url = args.no_more_url.clone();

    // coffee ready
    let ready = tokio::spawn(async move {
        let pin = Pin::new(args.ready);
        if let Err(e) = monitor_button(pin, &coffee_ready_url).await {
            eprintln!("Error: {}", e);
        }
    });

    let no_more = tokio::spawn(async move {
        let pin = Pin::new(args.no_more);
        if let Err(e) = monitor_button(pin, &coffee_no_more_url).await {
            eprintln!("Error: {}", e);
        }
    });

    let _ = (ready.await, no_more.await);
}

async fn monitor_button(pin: Pin, url: &str) -> sysfs_gpio::Result<()> {
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

                    send_slack_message(url).await;

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
