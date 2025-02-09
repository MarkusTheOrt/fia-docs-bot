use serenity::{
    all::{
        CacheHttp, CommandInteraction, CreateCommand,
        EditInteractionResponse, Permissions,
    },
    futures::future::join_all,
};
use tracing::{error, info};

use super::set;

pub fn register() -> CreateCommand {
    CreateCommand::new("sync")
        .description("Sync commands")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn run(
    ctx: impl CacheHttp,
    cmd: CommandInteraction,
) -> Result<(), serenity::Error> {
    info!("Running!");
    cmd.defer_ephemeral(&ctx).await?;
    {
        let mut fut = vec![];

        if let Ok(commands) = ctx.http().get_global_commands().await {
            for command in commands {
                fut.push(ctx.http().delete_global_command(command.id));
            }
        }

        for fut in join_all(fut).await {
            if let Err(why) = fut {
                error!("removing command: {why}");
            }
        }
    }

    {
        if let Err(why) =
            ctx.http().create_global_command(&set::register()).await
        {
            error!("Error registering command: {why}");
        }
    }

    cmd.edit_response(
        ctx,
        EditInteractionResponse::new().content("Synced commands!"),
    )
    .await?;

    Ok(())
}
