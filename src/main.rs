use clap::Parser;
use slack_morphism::{
    errors::SlackClientError,
    prelude::{
        SlackApiChatPostMessageRequest, SlackApiReactionsAddRequest, SlackClientHyperConnector,
    },
    SlackApiToken, SlackClient, SlackClientHttpConnector, SlackMessageContent, SlackTs,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use sysfs_gpio::{Direction, Pin};
use tokio::{
    join,
    sync::watch::{self, Receiver, Sender},
};

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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let ready_client = SlackClient::new(SlackClientHyperConnector::new());
    let no_more_client = SlackClient::new(SlackClientHyperConnector::new());
    let token = SlackApiToken::new(args.slack_token.into());

    //let ready_client = slack_client.clone();
    //let no_more_client = slack_client.clone();

    let ready_token = token.clone();
    let no_more_token = token.clone();

    let (tx, rx) = watch::channel(None);

    let _ = tokio::join!(
        ready_coffee_button(args.ready, ready_client, ready_token, tx),
        no_more_coffee_button(args.no_more, no_more_client, no_more_token, rx)
    );

    //let ready = tokio::spawn(async move {
    //    if let Err(e) = ready_coffee_button(args.ready, ready_client, ready_token, tx).await {
    //        eprintln!("Error: {}", e);
    //    };
    //    println!("Ready task finished");
    //});
    //
    //let no_more = tokio::spawn(async move {
    //    if let Err(e) = no_more_coffee_button(args.no_more, no_more_client, no_more_token, rx).await
    //    {
    //    };
    //    println!("No more task finished");
    //});
}

async fn ready_coffee_button<SCHC>(
    pin_num: u64,
    ready_client: SlackClient<SCHC>,
    ready_token: SlackApiToken,
    tx: Sender<Option<SlackTs>>,
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
        // TODO: emulate this on laptop using keyboard events as opposed to listening to GPIO pins
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
    rx: Receiver<Option<SlackTs>>,
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
    tx: &Sender<Option<SlackTs>>,
) -> Result<(), SlackClientError>
where
    SCHC: SlackClientHttpConnector + Send + Sync,
{
    println!("Sending message");
    let session = client.open_session(token);
    let post_msg_req = SlackApiChatPostMessageRequest::new(
        "#test-bots-here".into(),
        SlackMessageContent::new().with_text("☕".to_owned()),
    );

    //let post_msg_resp = session.chat_post_message(&post_msg_req).await?; // BUG: Gets stuck

    //match tokio::time::timeout(
    //    Duration::from_secs(5),
    //    session.chat_post_message(&post_msg_req),
    //)
    //.await
    //{
    //    Err(_) => {
    //        println!("did not receive value within 5s");
    //        Ok(())
    //    }
    //    Ok(Err(error)) => {
    //        println!("error: {}", error);
    //        Ok(())
    //    }
    //    Ok(Ok(post_msg_resp)) => {
    //        println!(
    //            "Message sent! TS: {}, Channel: {}",
    //            post_msg_resp.ts, post_msg_resp.channel
    //        );
    //        tx.send_replace(Some(post_msg_resp.ts));
    //        Ok(())
    //    }
    //}

    match session.chat_post_message(&post_msg_req).await {
        Ok(post_msg_resp) => {
            println!(
                "Message sent! TS: {}, Channel: {}",
                post_msg_resp.ts, post_msg_resp.channel
            );
            tx.send_replace(Some(post_msg_resp.ts));
            Ok(())
        }
        Err(error) => {
            println!("error: {}", error);
            Ok(())
        }
    }
}

async fn add_reaction<SCHC>(
    client: SlackClient<SCHC>,
    token: &SlackApiToken,
    mut rx: Receiver<Option<SlackTs>>,
) -> Result<(), SlackClientError>
where
    SCHC: SlackClientHttpConnector + Send + Sync,
{
    println!("Adding reaction");
    let session = client.open_session(token);
     let last_message_ts = rx.wait_for(Option::is_some).await.unwrap().clone();
    //let last_message_ts = rx.borrow().clone();

    match last_message_ts {
        Some(ts) => {
            println!("Last message ts found: {}", ts);
            let add_reaction_req =
                SlackApiReactionsAddRequest::new("#test-bots-here".into(), "zero0".into(), ts);

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
