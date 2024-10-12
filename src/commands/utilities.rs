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
	category = "Utilities",
	ephemeral
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
	category = "Utilities",
	ephemeral
)]
pub async fn uptime(
	ctx: Context<'_>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<(), Error> {
	let ephemeral = ephemeral.unwrap_or(false);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

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
