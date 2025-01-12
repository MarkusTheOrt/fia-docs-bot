use std::sync::{Arc, Mutex};

use f1_bot_types::Series;
use serenity::{
    all::{
        ChannelType, CommandInteraction,
        CommandOptionType::{Channel, SubCommand},
        PartialChannel, ResolvedOption, ResolvedValue, Role,
    },
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed,
        CreateInteractionResponse, CreateInteractionResponseFollowup,
        CreateInteractionResponseMessage,
    },
    model::permissions::Permissions,
    prelude::Context,
};
use sqlx::{mysql::MySqlQueryResult, MySql, Pool};

use crate::event_manager::GuildCache;

pub fn register() -> CreateCommand {
    CreateCommand::new("settings")
        .description("Set up the FIA Documents Bot")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .set_options(vec![
            create_option(Series::F1),
            create_option(Series::F2),
            create_option(Series::F3),
        ])
}

fn create_option(series: Series) -> CreateCommandOption {
    CreateCommandOption::new(
        SubCommand,
        series.to_string(),
        "Settings for the series",
    )
    .add_sub_option(create_thread_option())
    .add_sub_option(create_channel_option())
    .add_sub_option(create_role_option())
}

fn create_channel_option() -> CreateCommandOption {
    CreateCommandOption::new(Channel, "channel", "Channel to post documents in")
        .channel_types(vec![ChannelType::Text])
        .required(false)
}

fn create_thread_option() -> CreateCommandOption {
    CreateCommandOption::new(
        serenity::all::CommandOptionType::Boolean,
        "threads",
        "Whether or not to use threads (default = true)",
    )
    .required(true)
}

fn create_role_option() -> CreateCommandOption {
    CreateCommandOption::new(
        serenity::all::CommandOptionType::Role,
        "notify_role",
        "Optional Role that will be notified using @Role",
    )
}

fn error_embed(
    title: &str,
    description: &str,
) -> CreateEmbed {
    CreateEmbed::new().title(title).description(description).color(0xFF0000)
}

pub async fn run(
    pool: &Pool<MySql>,
    ctx: &Context,
    cmd: CommandInteraction,
    cache: &Arc<Mutex<GuildCache>>,
) -> Result<(), serenity::Error> {
    if cmd.guild_id.is_none() {
        let builder = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().ephemeral(true).embed(
                error_embed(
                    "Error",
                    "Command can only be run in a guild / server.",
                ),
            ),
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
                "f1" => {
                    series_command(Series::F1, pool, &cmd, options, cache).await
                },
                "f2" => {
                    series_command(Series::F2, pool, &cmd, options, cache).await
                },
                "f3" => {
                    series_command(Series::F3, pool, &cmd, options, cache).await
                },
                _ => {
                    let builder = CreateInteractionResponseFollowup::new()
                        .ephemeral(true)
                        .embed(error_embed(
                            "Error",
                            "Error invalid series selected.",
                        ));
                    cmd.create_followup(ctx, builder).await?;
                    return Ok(());
                },
            };
            match rv {
                Err(why) => {
                    let builder = CreateInteractionResponseFollowup::new()
                        .embed(error_embed("Error", &why));
                    cmd.create_followup(ctx, builder).await?;
                    return Ok(());
                },
                Ok(s) => {
                    let builder = CreateInteractionResponseFollowup::new()
                        .embed(error_embed("Success", &s));
                    cmd.create_followup(ctx, builder).await?;
                    return Ok(());
                },
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

    Ok(())
}

async fn series_command<'a>(
    series: Series,
    pool: &Pool<MySql>,
    cmd: &CommandInteraction,
    options: Vec<ResolvedOption<'_>>,
    cache: &Arc<Mutex<GuildCache>>,
) -> Result<String, String> {
    let options = resolve_options(options);
    if options.is_none() {
        return Err("Failed to resolve command options.".to_owned());
    }
    let (channel, threads, role) = options.unwrap();
    let guild = cmd.guild_id.unwrap();
    let role_id = role.map(|role| role.id.get());

    let channel_id = channel.map(|channel| channel.id.get());
    // TODO: Write the results into our cache so we don't have to query the database every 5 seconds..u
    {
        let mut cache = cache.lock().unwrap();
        if let Some(guild) =
            cache.cache.iter_mut().find(|p| p.id == guild.get())
        {
            let series = match series {
                Series::F1 => &mut guild.f1,
                Series::F2 => &mut guild.f2,
                Series::F3 => &mut guild.f3,
                _ => panic!("F1Academy not Supported!"),
            };
            series.role = role_id;
            series.channel = channel.map(|channel| channel.id.get());
            series.use_threads = threads;
        }
    }

    match series_query(pool, series, channel_id, threads, role_id, guild.get())
        .await
    {
        Ok(_) => {
            if role_id.is_some() && channel.is_some() {
                Ok(format!(
                    r#"Updated settings for {series}
                notify_role <@&{}>
                channel <#{}>
                use threads: `{}`"#,
                    role_id.unwrap(),
                    channel.unwrap().id.get(),
                    threads
                ))
            } else if channel.is_some() {
                return Ok(format!(
                    r#"Updated settings for {series}
                       channel <#{}>
                       use threads: `{}`"#,
                    channel.unwrap().id.get(),
                    threads
                ));
            } else {
                return Ok(
                    "cleared channel, won't be notified anymore.".to_string()
                );
            }
        },
        Err(why) => Err(format!("Database Error: ```log\n{why}```")),
    }
}

async fn series_query(
    pool: &Pool<MySql>,
    series: Series,
    channel: Option<u64>,
    threads: bool,
    role: Option<u64>,
    guild: u64,
) -> Result<MySqlQueryResult, sqlx::Error> {
    match series {
        Series::F1 => {
            sqlx::query!(
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
            .await
        },
        Series::F2 => {
            sqlx::query!(
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
            .await
        },
        Series::F3 => {
            sqlx::query!(
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
            .await
        },
        _ => panic!("F1Academy not Supported!")
    }
}

fn resolve_options(
    options: Vec<ResolvedOption<'_>>
) -> Option<(Option<&PartialChannel>, bool, Option<&Role>)> {
    let mut it = options.into_iter();
    let threads = it.next().take();
    let channel = it.next().take();
    let role = it.next().take();
    if let (channel, Some(threads), role) = (channel, threads, role) {
        let channel = match channel {
            None => None,
            Some(data) => {
                if let ResolvedValue::Channel(channel) = data.value {
                    Some(channel)
                } else {
                    None
                }
            },
        };
        let role = match role {
            None => None,
            Some(data) => {
                if let ResolvedValue::Role(role) = data.value {
                    Some(role)
                } else {
                    None
                }
            },
        };
        if let ResolvedValue::Boolean(threads) = threads.value {
            return Some((channel, threads, role));
        }
    }
    None
}
