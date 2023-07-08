use serenity::{builder::CreateCommand, prelude::Context};




pub fn register() -> CreateCommand{

    return CreateCommand::new("settings")
        .description("Set up the FIA Documents Bot");
}
