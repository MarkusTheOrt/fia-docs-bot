use serenity::{
    builder::CreateApplicationCommandOption,
    model::prelude::application_command::ApplicationCommandInteraction, prelude::Context,
};

pub mod ping;
pub mod set_channel;
pub mod set_role;
pub mod unset_channel;
pub mod unset_role;
pub mod stop_crawler;

pub fn set_options(
    option: &mut CreateApplicationCommandOption,
) -> &mut CreateApplicationCommandOption {
    return option
        .add_string_choice("Formula 1", "f1")
        .add_string_choice("Formula 2", "f2")
        .add_string_choice("Formula 3", "f3")
        .add_string_choice("World Rally Championship (WRC)", "wrc")
        .add_string_choice("World RX", "wrx");
}

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
