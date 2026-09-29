use bonehead::Error;
use bonehead::cron::monday_loop;
use bonehead::fantrax::get_standings;
use bonehead::leaderboard::{LeaderboardRow, fetch_avatar_bytes, generate_leaderboard_image};
use dashmap::DashMap;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{FullEvent, ReactionType, UserId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone)]
struct UserMetadata {
    reaction: Option<ReactionType>,
    fantasy_team_name: Option<String>,
}

#[derive(Default)]
struct Data {
    users: DashMap<UserId, UserMetadata>,
}
type Ctx<'a> = poise::Context<'a, Data, Error>;

/// Register something
#[poise::command(slash_command, subcommands("register_emoji", "register_fantasy"))]
async fn register(_ctx: Ctx<'_>) -> Result<(), Error> {
    // this body only runs if someone invokes the bare parent with no subcommand,
    // which slash commands with subcommands generally won't allow —
    // Discord requires picking one of the listed subcommands
    Ok(())
}

/// Register a reaction emoji
#[poise::command(slash_command, rename = "emoji")]
async fn register_emoji(
    ctx: Ctx<'_>,
    #[description = "Emoji to register"] emoji: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id();
    let reaction = match resolve_emoji(ctx.serenity_context(), guild_id, &emoji) {
        Ok(r) => r,
        Err(msg) => {
            ctx.say(msg).await?;
            return Ok(()); // stop here, don't fall through to inserting anything
        }
    };

    ctx.data()
        .users
        .entry(ctx.author().id)
        .or_default()
        .reaction = Some(reaction.clone());
    save_data(ctx.data()).await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("Registered reaction: {reaction}"))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

/// Register a fantasy team name
#[poise::command(slash_command, rename = "fantasy")]
async fn register_fantasy(
    ctx: Ctx<'_>,
    #[description = "Your fantasy team name"] fantasy_team_name: String,
) -> Result<(), Error> {
    ctx.data()
        .users
        .entry(ctx.author().id)
        .or_default()
        .fantasy_team_name = Some(fantasy_team_name.clone());
    save_data(ctx.data()).await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("Registered team name: {fantasy_team_name}"))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

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

fn find_team_owner(data: &Data, team_name: &str) -> Option<UserId> {
    data.users
        .iter()
        .find(|entry| entry.fantasy_team_name.as_deref() == Some(team_name))
        .map(|entry| *entry.key())
}

#[poise::command(prefix_command)]
async fn fantasy(ctx: Ctx<'_>) -> Result<(), Error> {
    let standings = get_standings().await.unwrap();

    // build rows: resolve owner + fetch avatar bytes for each team
    let mut rows = Vec::new();
    for team in &standings {
        let owner_id = find_team_owner(ctx.data(), &team.team_name);

        let (owner_display, avatar_bytes) = match owner_id {
            Some(user_id) => {
                let user = user_id.to_user(ctx.serenity_context()).await?;
                let bytes = fetch_avatar_bytes(&user.face()).await;
                (format!("@{}", user.name), bytes)
            }
            None => ("Unclaimed".to_string(), None),
        };

        rows.push(LeaderboardRow {
            rank: team.rank,
            team_name: team.team_name.clone(),
            record: team.points.clone(),
            owner_display,
            avatar_bytes,
        });
    }

    let png_bytes = generate_leaderboard_image(&rows)?;

    let role = serenity::RoleId::new(1554000251579539596);

    let attachment = serenity::CreateAttachment::bytes(png_bytes, "leaderboard.png");
    let embed = serenity::CreateEmbed::new()
        .title("🏆 Fantasy Standings")
        .image("attachment://leaderboard.png")
        .color(0x5865F2);
    let reply = poise::CreateReply::default()
        .content(format!("<@&{}>", role.get()))
        .embed(embed)
        .attachment(attachment)
        .allowed_mentions(serenity::CreateAllowedMentions::new().roles([role]));

    // let embed = serenity::CreateEmbed::new()
    //     .title("🏆 Fantasy Standings <@1554000251579539596>")
    //     .image("attachment://leaderboard.png")
    //     .color(0x5865F2);
    // let reply = poise::CreateReply::default()
    //     .embed(embed)
    //     .attachment(attachment);

    ctx.send(reply).await?;
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

// #[poise::command(prefix_command, rename = "emoji")]
// async fn set_emoji(
//     ctx: Ctx<'_>,
//     #[description = "User to trigger on when mentioned"] user: serenity::User,
//     #[description = "Emoji to react with"] emoji: String,
// ) -> Result<(), Error> {
//     let guild_id = ctx.guild_id();
//     let reaction = match resolve_emoji(ctx.serenity_context(), guild_id, &emoji) {
//         Ok(r) => r,
//         Err(msg) => {
//             ctx.say(msg).await?;
//             return Ok(()); // stop here, don't fall through to inserting anything
//         }
//     };
//
//     // context for acquiring write lock
//     {
//         let mut map = ctx.data().user_reactions.write().await;
//         map.insert(user.id, reaction.clone());
//     }
//     save_data(ctx.data()).await?;
//
//     Ok(())
// }

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

async fn save_data(data: &Data) -> Result<(), Error> {
    let json = serde_json::to_string_pretty(&data.users)?;
    tokio::fs::write("data.json", json).await?;
    Ok(())
}

async fn load_data() -> Data {
    match tokio::fs::read_to_string("data.json").await {
        Ok(json) => {
            let users: DashMap<UserId, UserMetadata> =
                serde_json::from_str(&json).unwrap_or_default();
            Data { users }
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
            commands: vec![fantasy(), register(), me()],
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

                let http = ctx.http.clone();
                let channel = serenity::ChannelId::new(1554313505887363082); // #fantasy chat
                tokio::spawn(monday_loop(http, channel));

                let guild_id = serenity::GuildId::new(239205378406088704);
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
