use std::collections::HashMap;

use bonehead::fantrax::get_standings;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{FullEvent, ReactionType, UserId};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

struct Data {
    user_reactions: RwLock<HashMap<UserId, ReactionType>>,
}
impl Data {
    fn default() -> Self {
        Data {
            user_reactions: RwLock::new(HashMap::new()),
        }
    }
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Ctx<'a> = poise::Context<'a, Data, Error>;

#[poise::command(prefix_command)]
async fn fantasy(ctx: Ctx<'_>) -> Result<(), Error> {
    let standings = get_standings().await.unwrap();

    let message = standings.iter().fold(
        String::from("```\nRank  Team                    Points\n"),
        |mut message, team| {
            message.push_str(&format!(
                "{:<5} {:<24} {}\n",
                team.rank, team.team_name, team.points,
            ));
            message
        },
    ) + "```";

    ctx.say(message).await?;
    Ok(())
}

fn resolve_emoji(
    ctx: &serenity::Context,
    guild_id: Option<serenity::GuildId>,
    input: &str,
) -> Result<ReactionType, String> {
    let trimmed = input.trim();

    // cheap checks first: parse as unicode emoji or full <a:name:id> custom syntax
    if let Ok(reaction) = ReactionType::try_from(trimmed) {
        match &reaction {
            ReactionType::Unicode(emoji_str) if emojis::get(emoji_str).is_some() => {
                return Ok(reaction);
            }
            ReactionType::Unicode(_) => {} // not a real unicode emoji, fall through to guild lookup
            _ => return Ok(reaction),      // custom <a:name:id> syntax parsed successfully
        }
    }

    // fall back to guild custom emoji lookup by bare name (for non-Nitro users)
    if let Some(guild_id) = guild_id {
        if let Some(emoji) = find_emoji_by_name(ctx, guild_id, trimmed) {
            return Ok(emoji.into());
        }
    }

    Err(format!("'{trimmed}' is not a valid emoji"))
}
fn find_emoji_by_name(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    name: &str,
) -> Option<serenity::Emoji> {
    let guild = ctx.cache.guild(guild_id)?;
    guild
        .emojis
        .values()
        .find(|e| e.name.eq_ignore_ascii_case(name))
        .cloned()
}

#[poise::command(prefix_command, rename = "emoji")]
async fn set_emoji(
    ctx: Ctx<'_>,
    #[description = "User to trigger on when mentioned"] user: serenity::User,
    #[description = "Emoji to react with"] emoji: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id();
    let reaction = match resolve_emoji(ctx.serenity_context(), guild_id, &emoji) {
        Ok(r) => r,
        Err(msg) => {
            ctx.say(msg).await?;
            return Ok(()); // stop here, don't fall through to inserting anything
        }
    };

    // context for acquiring write lock
    {
        let mut map = ctx.data().user_reactions.write().await;
        map.insert(user.id, reaction.clone());
    }
    save_data(ctx.data()).await?;

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
            let user_reactions = data.user_reactions.read().await;
            for mentioned in &new_message.mentions {
                if let Some(reaction) = user_reactions.get(&mentioned.id) {
                    new_message.react(ctx, reaction.clone()).await?;
                }
            }
            drop(user_reactions);
        }
        _ => {}
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Default)]
struct PersistedData {
    user_reactions: HashMap<UserId, ReactionType>,
}

async fn save_data(data: &Data) -> Result<(), Error> {
    let user_reactions = data.user_reactions.read().await.clone();
    let persisted = PersistedData { user_reactions };
    let json = serde_json::to_string_pretty(&persisted)?;
    tokio::fs::write("reactions.json", json).await?;
    Ok(())
}

async fn load_data() -> Data {
    match tokio::fs::read_to_string("reactions.json").await {
        Ok(json) => {
            let persisted: PersistedData = serde_json::from_str(&json).unwrap_or_default();
            Data {
                user_reactions: RwLock::new(persisted.user_reactions),
            }
        }
        Err(_) => Data::default(),
    }
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
            commands: vec![fantasy(), set_emoji()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            event_handler: |framework, event| Box::pin(event_handler(framework, event)),
            ..Default::default()
        })
        .setup(|_ctx, ready, _framework| {
            Box::pin(async move {
                println!("{} is connected!", ready.user.name);
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
