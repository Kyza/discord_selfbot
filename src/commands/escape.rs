use anyhow::Result;
use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

use crate::{helpers::escape_markdown, config::Context};

/// Escapes basic markdown characters.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn escape(
	ctx: Context<'_>,
	#[description = "The text to escape."] text: String,
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

	let text = escape_markdown(&text);

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.content(format!("```\n{}\n```", text))
		.ephemeral(ephemeral);

	ctx.send(reply).await?;
	Ok(())
}
