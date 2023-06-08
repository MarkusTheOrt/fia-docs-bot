use serenity::{
    model::{prelude::{application_command::ApplicationCommandInteraction, command::Command}, Permissions},
    prelude::Context,
};

pub async fn run(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), serenity::Error> {
    return command
        .create_interaction_response(&ctx, |msg| {
            return msg.interaction_response_data(|msg| {
                return msg.content(format!("Pong: Command id: `{}`", command.id));
            });
        })
        .await;
}

pub async fn register(ctx: &Context) -> Result<Command, serenity::Error>{
    return Command::create_global_application_command(ctx, |cmd| {
        return cmd
            .name("ping")
            .description("Check if the bot is running.")
            .default_member_permissions(Permissions::ADMINISTRATOR);
    }).await;
}
