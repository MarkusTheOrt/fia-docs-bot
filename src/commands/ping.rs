
use serenity::{model::prelude::application_command::ApplicationCommandInteraction, prelude::Context};



pub async fn run(ctx: &Context, command: &ApplicationCommandInteraction) -> Result<(), serenity::Error> {
    return command.create_interaction_response(&ctx, |msg| {
        return msg.interaction_response_data(|msg| {
            return msg.content(format!("Pong: Command id: `{}`", command.id));
        });
    }).await;
}
