use anyhow::Result;
use humansize::{format_size, DECIMAL};
use inline_format::format;
use memory_stats::memory_stats;
use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

use crate::config::Context;

/// Shows the bot's memory usage.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn memory(
	ctx: Context<'_>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let ephemeral = ephemeral.unwrap_or(false);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

	let content = if let Some(usage) = memory_stats() {
		format!(
			"Physical Memory: ",
			format_size(usage.physical_mem, DECIMAL),
			"\nVirtual Memory: ",
			format_size(usage.virtual_mem, DECIMAL),
		)
	} else {
		"Failed to get the current memory usage.".to_string()
	};

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.content(content)
		.ephemeral(ephemeral);

	ctx.send(reply).await?;
	Ok(())
}
