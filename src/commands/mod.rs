use serenity::{
    all::CommandInteraction,
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    prelude::Context,
};

pub mod set;

pub async fn unimplemented(
    ctx: &Context,
    command: CommandInteraction,
) -> Result<(), serenity::Error> {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .ephemeral(true)
            .content("Bot is currently in Maintenance Mode!"),
    );
    command.create_response(ctx, resp).await
}
