use libsql::Connection;
use serenity::all::{
    CacheHttp, CommandInteraction, CreateCommand, CreateEmbed,
    CreateInteractionResponseMessage, EditInteractionResponse, Permissions,
};

use crate::{
    database::{
        create_new_thread, fetch_guild_by_discord_id,
        fetch_latest_event_by_series, fetch_thread_for_guild_and_event,
    },
    error::Result,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("check-repost")
        .dm_permission(false)
        .description("Checks and posts the newest event if not posted.")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn run(
    conn: &Connection,
    http: &impl CacheHttp,
    cmd: CommandInteraction,
) -> Result {
    _ = conn;
    let Some(guild_id) = cmd.guild_id else {
        cmd.create_response(
            http,
            serenity::all::CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Can only be done in the context of a guild."),
            ),
        )
        .await?;
        return Ok(());
    };
    cmd.defer_ephemeral(http).await?;

    let Some(ev) =
        fetch_latest_event_by_series(conn, f1_bot_types::Series::F1).await?
    else {
        cmd.edit_response(
            http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new()
                    .title("Error")
                    .description("No Event found."),
            ),
        )
        .await?;
        return Ok(());
    };
    let Some(guild) = fetch_guild_by_discord_id(conn, guild_id).await? else {
        cmd.edit_response(
            http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new().title("Error").description("Invalid Guild"),
            ),
        )
        .await?;
        return Ok(());
    };

    if fetch_thread_for_guild_and_event(conn, guild.id, ev.id as i64)
        .await?
        .is_none()
    {
        create_new_thread(conn, http, &guild, &ev).await?;
        cmd.edit_response(
            http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new().title("Command still WIP").description(
                    format!("Created Thread for event ```rs\n{ev:#?}```"),
                ),
            ),
        )
        .await?;
    } else {
        cmd.edit_response(
            http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new()
                    .title("Info")
                    .description("Your guild already has a thread."),
            ),
        )
        .await?;
    }

    Ok(())
}
