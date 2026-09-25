use std::env;

use bonehead::edit_distance::close_enough;
use bonehead::fantrax::get_standings;
use serenity::all::{EmojiId, ReactionType};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Bonehead's own ID. Ignore own messages to prevent infinite loop
        if msg.author.id.to_string() == "1120914419875053638" {
            return;
        }

        if msg.content == "!fantasy" {
            let standings = get_standings().await.unwrap();

            let message = standings.iter().fold(
                String::from("```\nRank  Team                    Points\n"),
                |mut message, team| {
                    message.push_str(&format!(
                        "{:<5} {:<24} {:.1}\n",
                        team.rank, team.team_name, team.points,
                    ));
                    message
                },
            ) + "```";

            if let Err(why) = msg.channel_id.say(&ctx.http, message).await {
                println!("Error sending Discord message: {:?}", why);
            }
        }

        // if msg.content == "!embed" {
        //     let embed = CreateEmbed::new()
        //         .title("Embed Title")
        //         .description("Sample embed")
        //         .image("https://www.pngfind.com/pngs/m/52-527995_the-most-epic-meme-on-the-planet-png.png")
        //         .color(0x00ff00);

        //     let message = CreateMessage::new().embed(embed);
        //     if let Err(_why) = msg.channel_id.send_message(&ctx.http, message).await {
        //         // do nothing
        //     }
        // }

        // if msg.content == "!hello" && msg.channel_id.to_string() == "1547789542377914378" {
        //     if let Err(why) = msg
        //         .channel_id
        //         .say(&ctx.http, format!("Hello, {}", msg.author))
        //         .await
        //     {
        //         println!("Error sending message: {why:?}");
        //     }
        // }
        // if msg.content == "!revive" && msg.channel_id.to_string() == "1547789542377914378" {
        //     if let Err(why) = msg.channel_id.say(&ctx.http, "Hello, @here").await {
        //         println!("Error sending message: {why:?}");
        //     }
        // }

        if msg
            .content
            .split(" ")
            .any(|word| close_enough("game", word))
        {
            if let Err(_why) = msg.channel_id.say(
                &ctx.http,
                "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExMjhpYXgxMHA2M2hvZWgwM29yM3YwNGdoYTdlanR4YWtmMzZwZXdhcSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/yA4oraHXhwqNHpELV1/giphy.gif"
            ).await {
                // ignore
            }
        }

        if msg
            .content
            .split(" ")
            .any(|word| close_enough("matthew", word))
        {
            if let Err(_why) = msg.react(&ctx.http, '🐐').await {
                println!("Error");
            }
        }

        if msg
            .content
            .split(" ")
            .any(|word| close_enough("kyle", word) || word == "<@237764884127940618>")
        {
            let reaction = ReactionType::Custom {
                animated: false,
                id: EmojiId::new(1552834871267823656),
                name: Some("ricecat".to_string()),
            };
            if let Err(why) = msg.react(&ctx.http, reaction).await {
                println!("{:?}", why);
            }
        }

        if msg
            .content
            .split(" ")
            .any(|word| close_enough("attila", word) || word == "<@184453911980015616>")
        {
            if let Err(_why) = msg.react(&ctx.http, '😈').await {
                println!("Error");
            }
        }

        if msg
            .content
            .split(" ")
            .any(|word| close_enough("zack", word) || word == "<@230826525732241409>")
        {
            if let Err(_why) = msg.react(&ctx.http, '🔥').await {
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
