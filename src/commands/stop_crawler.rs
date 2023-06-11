use std::time::Duration;

use serenity::{
    model::{
        prelude::{command::Command, Activity},
        Permissions,
    },
    prelude::*,
};

use crate::event_manager::BotEvents;

pub async fn register(ctx: &Context) -> Result<Command, serenity::Error> {
    return Command::create_global_application_command(ctx, |cmd| {
        return cmd
            .name("stop-crawler")
            .description("Only botowner can use this command.")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .dm_permission(true);
    })
    .await;
}

pub async fn run(
    crawler: &BotEvents,
    ctx: &Context,
) -> Result<(), serenity::Error> {
    {
        let mut enabled = crawler.f1_crawler_enabled.lock().await;
        *enabled = false;
    }
    ctx.set_activity(Activity::listening("CRAWLER DISABLED!"))
        .await;
    return Ok(());
}
