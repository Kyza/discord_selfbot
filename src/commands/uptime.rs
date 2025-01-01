use anyhow::Error;

use crate::types::Context;

/// Tells you how long the bot has been up for.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
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
