use clap::Parser;
use slack_morphism::{
    errors::SlackClientError,
    prelude::{
        SlackApiChatPostMessageRequest, SlackApiReactionsAddRequest, SlackClientHyperConnector,
    },
    SlackApiToken, SlackChannelId, SlackClient, SlackClientHttpConnector, SlackMessageContent,
    SlackTs,
};
use std::time::{Duration, Instant};
use sysfs_gpio::{Direction, Pin};
use tokio::sync::watch::{self, Receiver, Sender};

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

#[derive(Clone)]
struct LastMessage {
    ts: SlackTs,
    channel: SlackChannelId,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let ready_client = SlackClient::new(SlackClientHyperConnector::new());
    let no_more_client = SlackClient::new(SlackClientHyperConnector::new());
    let token = SlackApiToken::new(args.slack_token.into());

    let ready_token = token.clone();
    let no_more_token = token.clone();

    let (tx, rx) = watch::channel(None);

    let ready = tokio::spawn(async move {
        if let Err(e) = ready_coffee_button(args.ready, ready_client, ready_token, tx).await {
            eprintln!("Error: {}", e);
        };
        println!("Ready task finished");
    });

    let no_more = tokio::spawn(async move {
        if let Err(e) = no_more_coffee_button(args.no_more, no_more_client, no_more_token, rx).await
        {
            eprintln!("Error: {}", e);
        };
        println!("No more task finished");
    });

    println!("Ready to report coffee! Press green button to report coffee ready, red button to report no more coffee");

    let _ = (ready.await, no_more.await);
}

async fn ready_coffee_button<SCHC>(
    pin_num: u64,
    ready_client: SlackClient<SCHC>,
    ready_token: SlackApiToken,
    tx: Sender<Option<LastMessage>>,
) -> anyhow::Result<()>
where
    SCHC: SlackClientHttpConnector + Send + Sync + Clone,
{
    let pin = Pin::new(pin_num);
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
                    if elapsed.as_secs() < 5 {
                        continue;
                    }

                    send_message(ready_client.clone(), &ready_token, &tx).await?;

                    now = Instant::now();
                }
            }
            None => (),
        }
    }
}

async fn no_more_coffee_button<SCHC>(
    pin_num: u64,
    no_more_client: SlackClient<SCHC>,
    no_more_token: SlackApiToken,
    rx: Receiver<Option<LastMessage>>,
) -> anyhow::Result<()>
where
    SCHC: SlackClientHttpConnector + Send + Sync + Clone,
{
    let pin = Pin::new(pin_num);
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
                    if elapsed.as_secs() < 5 {
                        continue;
                    }

                    add_reaction(no_more_client.clone(), &no_more_token, rx.clone()).await?;

                    now = Instant::now();
                }
            }
            None => (),
        }
    }
}

async fn send_message<SCHC>(
    client: SlackClient<SCHC>,
    token: &SlackApiToken,
    tx: &Sender<Option<LastMessage>>,
) -> Result<(), SlackClientError>
where
    SCHC: SlackClientHttpConnector + Send + Sync,
{
    println!("Sending message");
    let session = client.open_session(token);
    let post_msg_req = SlackApiChatPostMessageRequest::new(
        "#amsterdam".into(),
        SlackMessageContent::new().with_text("☕".to_owned()),
    );

    match session.chat_post_message(&post_msg_req).await {
        Ok(post_msg_resp) => {
            println!(
                "Message sent! TS: {}, Channel: {}",
                post_msg_resp.ts, post_msg_resp.channel
            );
            tx.send_replace(Some(LastMessage {
                ts: post_msg_resp.ts,
                channel: post_msg_resp.channel,
            }));
            Ok(())
        }
        Err(error) => {
            eprintln!("Error posting a message: {}", error);
            Ok(())
        }
    }
}

async fn add_reaction<SCHC>(
    client: SlackClient<SCHC>,
    token: &SlackApiToken,
    rx: Receiver<Option<LastMessage>>,
) -> Result<(), SlackClientError>
where
    SCHC: SlackClientHttpConnector + Send + Sync,
{
    println!("Adding reaction");
    let session = client.open_session(token);
    let last_message = rx.borrow().clone();

    match last_message {
        Some(message) => {
            println!("Last message ts found: {}", message.ts);
            let add_reaction_req =
                SlackApiReactionsAddRequest::new(message.channel, "zero0".into(), message.ts);

            println!("Adding reaction to message: {:?}", add_reaction_req);

            let add_reaction_resp = session.reactions_add(&add_reaction_req).await?;
            println!("Reaction added: {:?}", add_reaction_resp);
            Ok(())
        }
        None => {
            println!("No last message ts found, can't add reaction, skipping");
            Ok(())
        }
    }
}
