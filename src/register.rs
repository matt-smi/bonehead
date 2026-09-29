use crate::shared::{Ctx, Data, Error, save_data};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{ReactionType, UserId};

/// Register something
#[poise::command(slash_command, subcommands("register_emoji", "register_fantasy"))]
pub async fn register(_ctx: Ctx<'_>) -> Result<(), Error> {
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
    if find_team_owner(ctx.data(), &fantasy_team_name).is_some() {
        ctx.send(
            poise::CreateReply::default()
                .content(format!(
                    "Someone else has already registered the name _{fantasy_team_name}_!"
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.data()
        .users
        .entry(ctx.author().id)
        .or_default()
        .fantasy_team_name = Some(fantasy_team_name.clone());
    save_data(ctx.data()).await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("Registered team name: _{fantasy_team_name}_"))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

pub fn find_team_owner(data: &Data, team_name: &str) -> Option<UserId> {
    data.users
        .iter()
        .find(|entry| entry.fantasy_team_name.as_deref() == Some(team_name))
        .map(|entry| *entry.key())
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
