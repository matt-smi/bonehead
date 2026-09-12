use std::env;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!hello" && msg.channel_id.to_string() == "1547789542377914378" {
            if let Err(why) = msg
                .channel_id
                .say(&ctx.http, format!("Hello, {}", msg.author))
                .await
            {
                println!("Error sending message: {why:?}");
            }
        }
        if msg.content == "!revive" && msg.channel_id.to_string() == "1547789542377914378" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Hello, @here").await {
                println!("Error sending message: {why:?}");
            }
        }
        if msg.content.to_lowercase().contains("games") {
            if let Err(_why) = msg.channel_id.say(&ctx.http, "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExMjhpYXgxMHA2M2hvZWgwM29yM3YwNGdoYTdlanR4YWtmMzZwZXdhcSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/yA4oraHXhwqNHpELV1/giphy.gif").await {
                //
            }
        }
        if msg.content.to_lowercase().contains("zack") {
            if let Err(_why) = msg.react(&ctx.http, '🔥').await {
                println!("Error");
            }
            if let Err(_why) = msg
                .channel_id
                .say(&ctx.http, format!("Wassup <@{}>", 230826525732241409i64))
                .await
            {
                // no op
            }
        }
        if msg.content.to_lowercase().contains("matthew")
            || msg.content.to_lowercase().contains("<@235962572912721922>")
        {
            if let Err(_why) = msg.react(&ctx.http, '🐐').await {
                println!("Error");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    dotenvy::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
