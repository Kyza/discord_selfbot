use anyhow::Error;

use crate::config::Context;

/// Shows the help menu. Ephemeral by default.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn help(
	ctx: Context<'_>,
	#[description = "Specific command to show help about."]
	#[autocomplete = "poise::builtins::autocomplete_command"]
	command: Option<String>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<(), Error> {
	let ephemeral = ephemeral.unwrap_or(true);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

	poise::builtins::help(
		ctx,
		command.as_deref(),
		poise::builtins::HelpConfiguration {
			extra_text_at_bottom: "\
Type /help command for more info on a command.",
			ephemeral,
			..Default::default()
		},
	)
	.await?;
	Ok(())
}
