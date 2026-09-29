use bonehead::cron::monday_loop;
use bonehead::register::register;
use bonehead::shared::{Ctx, Data, Error, GENERAL_CHAT, GUILD_ID, load_data};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::FullEvent;

#[poise::command(prefix_command)]
async fn me(ctx: Ctx<'_>) -> Result<(), Error> {
    let (team_name, reaction) = ctx
        .data()
        .users
        .get(&ctx.author().id)
        .map(|u| (u.fantasy_team_name.clone(), u.reaction.clone()))
        .unwrap_or((None, None));

    let team_name = team_name.unwrap_or_else(|| "No team registered".to_string());
    let reaction = reaction
        .map(|r| r.to_string())
        .unwrap_or_else(|| "".to_string());

    let embed = serenity::CreateEmbed::new()
        .author(serenity::CreateEmbedAuthor::new(&ctx.author().name).icon_url(ctx.author().face()))
        .field("Fantasy Team", format!("_{}_", team_name), true)
        .field("Reaction", reaction, true)
        .color(0x57F287);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

async fn event_handler(
    framework: poise::FrameworkContext<'_, Data, Error>,
    event: &serenity::FullEvent,
) -> Result<(), Error> {
    let ctx = framework.serenity_context;
    let data = framework.user_data;

    match event {
        FullEvent::Message { new_message } => {
            // ignore bot messages to avoid loops
            if new_message.author.bot {
                return Ok(());
            }
            for mentioned in &new_message.mentions {
                let reaction = data
                    .users
                    .get(&mentioned.id)
                    .and_then(|m| m.reaction.clone());
                if let Some(reaction) = reaction {
                    new_message.react(ctx, reaction).await?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    dotenvy::dotenv().ok();
    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![register(), me()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            event_handler: |framework, event| Box::pin(event_handler(framework, event)),
            ..Default::default()
        })
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                println!("{} is connected!", ready.user.name);

                let data = load_data().await;

                let users_for_cron = data.users.clone(); // shallow clone
                let http = ctx.http.clone();
                let channel = serenity::ChannelId::new(GENERAL_CHAT);
                tokio::spawn(monday_loop(http, channel, users_for_cron));

                let guild_id = serenity::GuildId::new(GUILD_ID);
                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id)
                    .await?;

                Ok(load_data().await)
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client
        .expect("error creating client")
        .start()
        .await
        .expect("client error");
}
