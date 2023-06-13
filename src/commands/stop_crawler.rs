use std::time::Duration;

use serenity::{
    builder::CreateEmbed,
    model::{
        prelude::{application_command::ApplicationCommandInteraction, command::Command, Activity},
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
    })
    .await;
}

pub async fn run(
    crawler: &BotEvents,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), serenity::Error> {
    command.defer(ctx).await?;
    {
        let mut enabled = crawler.f1_crawler_enabled.lock().await;
        *enabled = false;
    }
    ctx.set_activity(Activity::listening("CRAWLER DISABLED!"))
        .await;

    command
        .edit_original_interaction_response(ctx, |msg| {
            let mut e1 = CreateEmbed::default();
            e1.url("https://google.com/")
                .author(|a| a.name("FIA Document"))
                .title("Example Document")
                .colour(0x002d5f)
                .thumbnail("https://static.ort.dev/fiadontsueme/fia_logo.png")
                .image("https://fia.ort.dev/647cc7d482a56d96fdaacb5a.jpg")
                .footer(|f| f.text("Date/time here"));
            let mut e2 = CreateEmbed::default();
            e2.image("https://fia.ort.dev/647cc71f82a56d96fdaacb59.jpg")
                .url("https://google.com/");
            let mut e3 = CreateEmbed::default();
            e3.image("https://fia.ort.dev/647cc50182a56d96fdaacb58.jpg")
                .url("https://google.com/");
            let mut e4 = CreateEmbed::default();
            e4.image("https://fia.ort.dev/647cc17b82a56d96fdaacb57.jpg")
                .url("https://google.com/");
            let mut e5 = CreateEmbed::default();
            e5.image("https://fia.ort.dev/647cb9b882a56d96fdaacb56.jpg")
                .url("https://google.com");
            return msg.add_embed(e1).add_embed(e2).add_embed(e3).add_embed(e4).add_embed(e5);
        })
        .await?;

    return Ok(());
}
