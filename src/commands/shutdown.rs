use serenity::all::{
    CommandInteraction, Context, CreateCommand,
    CreateInteractionResponse::Message, CreateInteractionResponseMessage,
    Permissions,
};

use crate::ShardManagerBox;

pub fn register() -> CreateCommand {
    CreateCommand::new("shutdown")
        .description("Gracefully exit the bot.")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn run(
    ctx: &Context,
    cmd: CommandInteraction,
) -> crate::error::Result {
    cmd.create_response(
        &ctx,
        Message(
            CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("But is shutting down now..."),
        ),
    )
    .await?;
    {
        let lock = ctx.data.write().await;
        if let Some(shards) = lock.get::<ShardManagerBox>() {
            shards.shutdown_all().await;
        }
    }
    Ok(())
}
