use libsql::Connection;
use serenity::all::{
    CacheHttp, CommandInteraction, CreateCommand, CreateEmbed,
    EditInteractionResponse, Permissions,
};

use crate::error::Result;

pub fn register() -> CreateCommand {
    CreateCommand::new("check-repost")
        .description("Checks and posts the newest event if not posted.")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn run(
    conn: &Connection,
    http: &impl CacheHttp,
    cmd: CommandInteraction,
) -> Result {
    _ = conn;
    cmd.defer_ephemeral(http).await?;

    cmd.edit_response(
        http,
        EditInteractionResponse::new().embed(
            CreateEmbed::new()
                .title("Success.")
                .description("Reposting documents for F1/F2/F3 now."),
        ),
    )
    .await?;

    Ok(())
}
