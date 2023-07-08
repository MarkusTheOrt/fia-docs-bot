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
            .content("Not implemented!"),
    );
    return command.create_response(ctx, resp).await;
}
