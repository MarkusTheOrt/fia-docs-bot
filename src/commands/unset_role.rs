use serenity::{prelude::*, model::{prelude::command::{Command, CommandOptionType}, Permissions}};

use super::set_options;




pub async fn register(ctx: &Context) -> Result<Command, serenity::Error> {
    return Command::create_global_application_command(ctx, |cmd| {
        return cmd
            .name("unset-role")
            .description("Remove the role that gets notified for a certain document.")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .create_option(|opt| {
                return set_options(opt)
                    .name("series")
                    .description("Clear the role for which series?")
                    .kind(CommandOptionType::String)
                    .required(true);
            });
    })
    .await;
}
