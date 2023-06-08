use serenity::{
    model::prelude::application_command::ApplicationCommandInteraction, prelude::Context,
};

pub mod ping;
pub mod set_channel;
pub mod unset_channel;

pub async fn unimplemented(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), serenity::Error> {
    return command
        .create_interaction_response(ctx, |msg| {
            return msg.interaction_response_data(|msg| {
                return msg.ephemeral(true).content("not implemented");
            });
        })
        .await;
}
