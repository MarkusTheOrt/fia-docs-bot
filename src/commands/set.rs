use serenity::{
    all::{
        ChannelType, CommandInteraction,
        CommandOptionType::{Channel, SubCommand},
        PartialChannel, ResolvedOption, ResolvedValue, Role,
    },
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    },
    model::permissions::Permissions,
    prelude::Context,
};
use sqlx::{mysql::MySqlQueryResult, MySql, Pool};

use crate::model::series::RacingSeries;

pub fn register() -> CreateCommand {
    return CreateCommand::new("settings")
        .description("Set up the FIA Documents Bot")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .set_options(vec![
            create_option(RacingSeries::F1),
            create_option(RacingSeries::F2),
            create_option(RacingSeries::F3),
        ]);
}

fn create_option(series: RacingSeries) -> CreateCommandOption {
    return CreateCommandOption::new(SubCommand, series, "Settings for the series")
        .add_sub_option(create_channel_option())
        .add_sub_option(create_thread_option())
        .add_sub_option(create_role_option());
}

fn create_channel_option() -> CreateCommandOption {
    return CreateCommandOption::new(Channel, "channel", "Channel to post documents in")
        .channel_types(vec![ChannelType::Text])
        .required(true);
}

fn create_thread_option() -> CreateCommandOption {
    return CreateCommandOption::new(
        serenity::all::CommandOptionType::Boolean,
        "threads",
        "Whether or not to use threads (default = true)",
    )
    .required(true);
}

fn create_role_option() -> CreateCommandOption {
    return CreateCommandOption::new(
        serenity::all::CommandOptionType::Role,
        "notify_role",
        "Optional Role that will be notified using @Role",
    );
}

fn error_embed(title: &str, description: &str) -> CreateEmbed {
    return CreateEmbed::new()
        .title(title)
        .description(description)
        .color(0xFF0000);
}

pub async fn run(
    pool: &Pool<MySql>,
    ctx: &Context,
    cmd: CommandInteraction,
) -> Result<(), serenity::Error> {
    if cmd.guild_id.is_none() {
        let builder = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .embed(error_embed(
                    "Error",
                    "Command can only be run in a guild / server.",
                )),
        );
        cmd.create_response(ctx, builder).await?;
        return Ok(());
    }

    cmd.defer_ephemeral(ctx).await?;
    let options = cmd.data.options();

    let subcommand = options.into_iter().next().take();
    if let Some(command) = subcommand {
        if let ResolvedValue::SubCommand(options) = command.value {
            let rv = match command.name {
                "f1" => series_command(RacingSeries::F1, pool, &cmd, options).await,
                "f2" => series_command(RacingSeries::F2, pool, &cmd, options).await,
                "f3" => series_command(RacingSeries::F2, pool, &cmd, options).await,
                _ => {
                    let builder = CreateInteractionResponseFollowup::new()
                        .ephemeral(true)
                        .embed(error_embed("Error", "Error invalid series selected."));
                    cmd.create_followup(ctx, builder).await?;
                    return Ok(());
                }
            };
            match rv {
                Err(why) => {
                    let builder =
                        CreateInteractionResponseFollowup::new().embed(error_embed("Error", &why));
                    cmd.create_followup(ctx, builder).await?;
                    return Ok(());
                }
                Ok(s) => {
                    let builder =
                        CreateInteractionResponseFollowup::new().embed(error_embed("Success", &s));
                    cmd.create_followup(ctx, builder).await?;
                    return Ok(());
                }
            };
        }
    } else {
        let builder = CreateInteractionResponseFollowup::new()
            .ephemeral(true)
            .embed(error_embed(
                "Unkown Error",
                "There was an error parsing your command data.",
            ));
        cmd.create_followup(ctx, builder).await?;
    }

    return Ok(());
}

async fn series_command<'a>(
    series: RacingSeries,
    pool: &Pool<MySql>,
    cmd: &CommandInteraction,
    options: Vec<ResolvedOption<'_>>,
) -> Result<String, String> {
    let options = resolve_options(options);
    if options.is_none() {
        return Err("Failed to resolve command options.".to_owned());
    }
    let (channel, threads, role) = options.unwrap();
    let guild = cmd.guild_id.unwrap();
    let role_id = match role {
        Some(role) => Some(role.id.get()),
        None => None,
    };
    match series_query(
        pool,
        series,
        channel.id.get(),
        threads,
        role_id,
        guild.get(),
    )
    .await
    {
        Ok(_) => {
            if role_id.is_some() {
                return Ok(format!(
                    r#"Updated settings for {series}
                notify_role <@&{}>
                channel <#{}>
                use threads: `{}`"#,
                    role_id.unwrap(),
                    channel.id.get(),
                    threads
                ));
            } else {
                return Ok(format!(
                    r#"Updated settings for {series}
                       channel <#{}>
                       use threads: `{}`"#,
                    channel.id.get(),
                    threads
                ));
            }
        }
        Err(why) => {
            return Err(format!("Database Error: ```log\n{why}```"));
        }
    }
}

async fn series_query(
    pool: &Pool<MySql>,
    series: RacingSeries,
    channel: u64,
    threads: bool,
    role: Option<u64>,
    guild: u64,
) -> Result<MySqlQueryResult, sqlx::Error> {
    match series {
        RacingSeries::F1 => {
            return sqlx::query!(
                r#"UPDATE guilds
                   SET f1_channel = ?,
                   f1_threads = ?,
                   f1_role = ?
                   WHERE id = ?"#,
                channel,
                threads,
                role,
                guild
            )
            .execute(pool)
            .await;
        }
        RacingSeries::F2 => {
            return sqlx::query!(
                r#"UPDATE guilds
                   SET f2_channel = ?,
                   f2_threads = ?,
                   f2_role = ?
                   WHERE id = ?"#,
                channel,
                threads,
                role,
                guild
            )
            .execute(pool)
            .await;
        }
        RacingSeries::F3 => {
            return sqlx::query!(
                r#"UPDATE guilds
                   SET f3_channel = ?,
                   f3_threads = ?,
                   f3_role = ?
                   WHERE id = ?"#,
                channel,
                threads,
                role,
                guild
            )
            .execute(pool)
            .await;
        }
    }
}

fn resolve_options<'a>(
    options: Vec<ResolvedOption<'a>>,
) -> Option<(&'a PartialChannel, bool, Option<&'a Role>)> {
    let mut it = options.into_iter();
    let channel = it.next().take();
    let threads = it.next().take();
    let role = it.next().take();
    if let (Some(channel), Some(threads), role) = (channel, threads, role) {
        let role = match role {
            None => None,
            Some(data) => {
                if let ResolvedValue::Role(role) = data.value {
                    Some(role)
                } else {
                    None
                }
            }
        };
        if let (ResolvedValue::Channel(channel), ResolvedValue::Boolean(threads)) =
            (channel.value, threads.value)
        {
            return Some((channel, threads, role));
        }
    }
    return None;
}
