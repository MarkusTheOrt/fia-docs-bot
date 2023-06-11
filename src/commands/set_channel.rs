use serenity::{
    model::{
        prelude::{
            application_command::{
                ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
            },
            command::{Command, CommandOptionType},
            ChannelType,
        },
        Permissions,
    },
    prelude::Context,
};
use sqlx::{MySql, Pool};

pub fn unwrap_option<'a>(
    option: Option<&'a CommandDataOption>,
) -> Option<&'a CommandDataOptionValue> {
    let data = match option.as_ref() {
        None => return None,
        Some(data) => data,
    };

    let data = match data.resolved.as_ref() {
        None => return None,
        Some(data) => data,
    };

    return Some(data);
}

pub async fn run(
    pool: &Pool<MySql>,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), serenity::Error> {
    command.defer_ephemeral(ctx).await?;

    if command.member.is_none() {
        command
            .edit_original_interaction_response(ctx, |msg| {
                return msg.content("This command can only ever be called in a guild/server.");
            })
            .await?;
        return Ok(());
    }

    let member = command.member.as_ref().unwrap();

    let member_perms = member.permissions(ctx)?;
    if !member_perms.administrator() {
        command
            .edit_original_interaction_response(ctx, |msg| {
                return msg
                    .content("You must have the `Adiminstrator` permissions to use this command!");
            })
            .await?;
        return Ok(());
    }

    let (channel, series) = match (
        unwrap_option(command.data.options.get(0)),
        unwrap_option(command.data.options.get(1)),
    ) {
        (Some(channel), Some(series)) => (channel, series),
        _ => {
            command
                .edit_original_interaction_response(ctx, |msg| {
                    return msg.content("Error parsing Command Options");
                })
                .await?;
            return Ok(());
        }
    };
    if let (CommandDataOptionValue::Channel(ch), CommandDataOptionValue::String(series)) =
        (channel, series)
    {
        let guild = command.guild_id.as_ref().unwrap().as_u64();

        let res = sqlx::query!(
            "UPDATE guilds SET channel = ? WHERE id = ?",
            ch.id.as_u64(),
            guild
        )
        .execute(pool)
        .await;

        if let Err(why) = res {
            command
                .edit_original_interaction_response(&ctx, |msg| {
                    return msg.content(format!("Error: {why}"));
                })
                .await?;
        }
        command
            .edit_original_interaction_response(&ctx, |msg| {
                return msg.embed(|embed| {
                    return embed
                        .description(format!("Set {series}-channel to <#{}>", ch.id.as_u64()))
                        .colour(0x00FF00);
                });
            })
            .await?;
    }

    return Ok(());
}

pub async fn register(ctx: &Context) -> Result<Command, serenity::Error> {
    return Command::create_global_application_command(ctx, |cmd| {
        return cmd
            .name("set-channel")
            .description("Set the Channel of where to post new documents for a specific series.")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .create_option(|opt| {
                return opt
                    .name("channel")
                    .description("The Text Channel where to post new threads/documents.")
                    .kind(CommandOptionType::Channel)
                    .required(true)
                    .channel_types(&vec![ChannelType::Text]);
            })
            .create_option(|opt| {
                return opt
                    .name("series")
                    .description("Documents of which racing series should be posted?")
                    .kind(CommandOptionType::String)
                    .required(true)
                    .add_string_choice("Formula 1", "f1");
            });
    })
    .await;
}
