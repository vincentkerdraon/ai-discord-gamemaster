use axum::{response::Html, routing::get, Router};
use serenity::{async_trait, model::gateway::Ready, prelude::*, Client};
use std::env;
use tracing::{debug, info};

struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // Initialize the tracing subscriber
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Create a new instance of the Client, logging in as a bot. The builder method
    // returns an error here if the token is invalid or other problems with the
    // bot are present.
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    info!("Create client discord...");
    info!(token);
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    let _axum_handle = tokio::spawn(async move {
        info!("Starting server...");

        // build our application with a route
        let app = Router::new().route("/", get(hello_world));

        // run it
        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

async fn hello_world() -> Html<&'static str> {
    Html("Hello, World!")
}
