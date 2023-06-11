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

use super::{set_channel::unwrap_option, set_options};

pub async fn register(ctx: &Context) -> Result<Command, serenity::Error> {
    return Command::create_global_application_command(ctx, |cmd| {
        return cmd
            .name("unset-channel")
            .description("Set the Channel of where to post new documents for a specific series.")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .create_option(|opt| {
                return set_options(opt)
                    .name("series")
                    .description("Which series should not be posted anymore?")
                    .kind(CommandOptionType::String)
                    .required(true)
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

    let user = match command.member.as_ref() {
        Some(user) => user,
        None => {
            command
                .edit_original_interaction_response(ctx, |msg| {
                    return msg.content("Userdata not found.");
                })
                .await?;
            return Ok(());
        }
    };
    {
        let perms = match user.permissions.as_ref() {
            Some(perms) => perms,
            None => {
                command
                    .edit_original_interaction_response(ctx, |msg| {
                        return msg.content("Permissions not found.");
                    })
                    .await?;
                return Ok(());
            }
        };

        if !perms.administrator() {
            command
                .edit_original_interaction_response(ctx, |msg| {
                    return msg.content("You need to be `Administrator` to use this.");
                })
                .await?;
            return Ok(());
        }
    }

    let guild_id = match command.guild_id.as_ref() {
        Some(guild_id) => guild_id,
        None => {
            command
                .edit_original_interaction_response(ctx, |msg| {
                    return msg.content("Command can only be run in a guild/server!");
                })
                .await?;
            return Ok(());
        }
    };

    if let Some(CommandDataOptionValue::String(series)) = unwrap_option(command.data.options.get(0)) {
        let query = sqlx::query!(
            "UPDATE guilds SET channel = NULL where id = ?",
            guild_id.as_u64()
        )
        .execute(pool)
        .await;

        match query {
            Ok(_) => {
                command
                    .edit_original_interaction_response(ctx, |msg| {
                        return msg.embed(|embed| {
                            return embed.description(format!("unset channel for `{series}`-series")).colour(0x00FF00);
                        });
                    })
                    .await?;
                return Ok(());
            }
            Err(_) => {
                command
                    .edit_original_interaction_response(ctx, |msg| {
                        return msg.embed(|embed| {
                            return embed.colour(0xFF0000).description("Database Error");
                        });
                    })
                    .await?;
                return Ok(());
            }
        }
    } else {
        command
            .edit_original_interaction_response(ctx, |msg| {
                return msg.content("Error in command options.");
            })
            .await?;
    }

    return Ok(());
}
