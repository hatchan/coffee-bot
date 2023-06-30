use clap::Parser;
use slack_morphism::{
    prelude::{SlackApiTestRequest, SlackClientHyperConnector},
    *,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use sysfs_gpio::{Direction, Pin};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    ready: u64,
    #[arg(short, long)]
    no_more: u64,
    #[arg(short, long, env)]
    slack_token: String,
}

enum CoffeeState {
    Ready,
    NoMore,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let slack_client = Arc::new(SlackClient::new(SlackClientHyperConnector::new()));
    let token = SlackApiToken::new(args.slack_token.into());

    //if let Err(e) = session
    //    .api_test(&SlackApiTestRequest::new().with_foo("Test".into()))
    //    .await
    //{
    //    eprintln!("Slack API connection error: {}", e);
    //    return;
    //}

    SlackStateWo
    let ready_client = slack_client.clone();
    let no_more_client = slack_client.clone();

    let ready_token = token.clone();
    let no_more_token = token.clone();


    // coffee ready
    let ready = tokio::spawn(async move {
        let pin = Pin::new(args.ready);
        if let Err(e) = monitor_button(CoffeeState::Ready, pin, ready_client, ready_token).await {
            eprintln!("Error: {}", e);
        }
    });

    let no_more = tokio::spawn(async move {
        let pin = Pin::new(args.no_more);
        if let Err(e) = monitor_button(CoffeeState::NoMore, pin, no_more_client, no_more_token).await {
            eprintln!("Error: {}", e);
        }
    });

    let _ = (ready.await, no_more.await);
}

async fn monitor_button<SCHC>(
    state: CoffeeState,
    pin: Pin,
    client: Arc<SlackClient<SCHC>>,
    token: SlackApiToken,
) -> sysfs_gpio::Result<()>
where
    SCHC: SlackClientHttpConnector + Send + Sync,
{
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

                    match state {
                        CoffeeState::Ready => send_message(client, &token).await,
                        CoffeeState::NoMore => add_reaction(client, &token).await,
                    }

                    // TODO: either send message or add reaction to last message


                    now = Instant::now();
                }
            }
            None => (),
        }
    }
}

async fn add_reaction<SCHC>(client: Arc<SlackClient<SCHC>>, token: &SlackApiToken) where SCHC: SlackClientHttpConnector + Send + Sync {
    todo!()
}

async fn send_message<SCHC>(client: Arc<SlackClient<SCHC>>, token: SlackApiToken) where SCHC: SlackClientHttpConnector + Send + Sync {
    todo!()
}


