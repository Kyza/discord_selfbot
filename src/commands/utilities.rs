use anyhow::Error;
use poise::serenity_prelude as serenity;

use crate::types::Context;

/// Links to the bot GitHub repo
// #[poise::command(
// 	slash_command,
// 	owners_only,
// 	track_edits,
// 	install_context = "User",
// 	interaction_context = "Guild|BotDm|PrivateChannel",
// 	category = "Utilities",
// 	discard_spare_arguments
// )]
// pub async fn source(ctx: Context<'_>) -> Result<(), Error> {
// 	ctx.say(
// 		"https://github.com/rust-community-discord/ferrisbot-for-discord",
// 	)
// 	.await?;
// 	Ok(())
// }

/// Show this menu.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	category = "Utilities"
)]
pub async fn help(
	ctx: Context<'_>,
	#[description = "Specific command to show help about"]
	#[autocomplete = "poise::builtins::autocomplete_command"]
	command: Option<String>,
) -> Result<(), Error> {
	let extra_text_at_bottom = "\
Type /help command for more info on a command.
You can edit your message to the bot and the bot will edit its response.";

	poise::builtins::help(
		ctx,
		command.as_deref(),
		poise::builtins::HelpConfiguration {
			extra_text_at_bottom,
			ephemeral: true,
			..Default::default()
		},
	)
	.await?;
	Ok(())
}

/// Tells you how long the bot has been up for.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	category = "Utilities"
)]
pub async fn uptime(ctx: Context<'_>) -> Result<(), Error> {
	let uptime = ctx.data().bot_start_time.elapsed();

	let div_mod = |a, b| (a / b, a % b);

	let seconds = uptime.as_secs();
	let (minutes, seconds) = div_mod(seconds, 60);
	let (hours, minutes) = div_mod(minutes, 60);
	let (days, hours) = div_mod(hours, 24);

	ctx.say(format!("Uptime: {days}d {hours}h {minutes}m {seconds}s"))
		.await?;

	Ok(())
}

/// Deletes the bot's messages for cleanup.
///
/// /cleanup [limit]
///
/// By default, only the most recent bot message is deleted (limit = 1).
///
/// Deletes the bot's messages for cleanup.
/// You can specify how many messages to look for. Only messages within the last 24 hours can be deleted.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	category = "Utilities",
	on_error = "crate::helpers::acknowledge_fail"
)]
pub async fn cleanup(
	ctx: Context<'_>,
	#[description = "Number of messages to delete"] num_messages: Option<
		usize,
	>,
) -> Result<(), Error> {
	let num_messages = num_messages.unwrap_or(1);

	let messages_to_delete = ctx
		.channel_id()
		.messages(&ctx, serenity::GetMessages::new())
		.await?
		.into_iter()
		.filter(|msg| {
			(msg.author.id == ctx.data().application_id)
				&& (*ctx.created_at() - *msg.timestamp).num_hours() < 24
		})
		.take(num_messages);

	ctx.channel_id()
		.delete_messages(&ctx, messages_to_delete)
		.await?;

	crate::helpers::acknowledge_success(ctx, "rustOk", '👌').await
}
