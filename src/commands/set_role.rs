use serenity::{
    model::{
        prelude::{
            application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
            command::{Command, CommandOptionType},
        },
        Permissions,
    },
    prelude::*,
};
use sqlx::{MySql, Pool};

use super::set_channel::unwrap_option;

pub async fn register(ctx: &Context) -> Result<Command, serenity::Error> {
    return Command::create_global_application_command(ctx, |cmd| {
        return cmd
            .name("set-role")
            .description("Set the Role to get notified with new documents.")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .create_option(|opt| {
                return opt
                    .name("role")
                    .description("The Notifications Role.")
                    .kind(CommandOptionType::Role)
                    .required(true);
            })
            .create_option(|opt| {
                return opt
                    .name("series")
                    .description("Role for which Series should be notified.")
                    .kind(CommandOptionType::String)
                    .required(true)
                    .add_string_choice("Formula 1", "f1");
            });
    })
    .await;
}

pub async fn run(
    pool: &Pool<MySql>,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), serenity::Error> {
    command.defer_ephemeral(ctx).await?;

    let guild_id = match command.guild_id {
        Some(guild_id) => guild_id,
        None => {
            command
                .edit_original_interaction_response(ctx, |msg| {
                    return msg.content("This command can only be used in a guild/server.");
                })
                .await?;
            return Ok(());
        }
    };

    {
        let user = match command.member.as_ref() {
            Some(user) => user,
            None => {
                command
                    .edit_original_interaction_response(ctx, |msg| {
                        return msg.content("Couldn't fetch userdata from command.");
                    })
                    .await?;
                return Ok(());
            }
        };

        let permissions = match user.permissions.as_ref() {
            Some(permissions) => permissions,
            None => {
                command
                    .edit_original_interaction_response(ctx, |msg| {
                        return msg.content("Couldn't fetch user-permissions from command.");
                    })
                    .await?;
                return Ok(());
            }
        };

        if !permissions.administrator() {
            command
                .edit_original_interaction_response(ctx, |msg| {
                    return msg.content(
                        "You need to have the `Administrator` permission to use this command.",
                    );
                })
                .await?;
            return Ok(());
        }
    }

    let (role, series) = match (
        unwrap_option(command.data.options.get(0)),
        unwrap_option(command.data.options.get(1)),
    ) {
        (Some(role), Some(series)) => (role, series),
        _ => {
            command
                .edit_original_interaction_response(ctx, |msg| {
                    return msg.content("Could not fetch command options.");
                })
                .await?;
            return Ok(());
        }
    };

    if let (CommandDataOptionValue::Role(role), CommandDataOptionValue::String(series)) =
        (role, series)
    {
        if let Err(why) = sqlx::query!("UPDATE guilds SET notify_role = ? WHERE id = ?", role.id.as_u64(), guild_id.as_u64())
            .execute(pool)
            .await
        {
            command
                .edit_original_interaction_response(ctx, |msg| {
                    return msg.content(format!("Database Error: ```{why}```"));
                })
                .await?;
            return Ok(());
        }

        command
            .edit_original_interaction_response(ctx, |msg| {
                return msg.content(format!(
                    "Set role for `{series}` to <@&{}>",
                    role.id.as_u64()
                ));
            })
            .await?;

        return Ok(());
    } else {
        command
            .edit_original_interaction_response(ctx, |msg| {
                return msg.content("Couldn't resolve command options.");
            })
            .await?;
        return Ok(());
    }
}
