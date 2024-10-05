use anyhow::Result;
use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

use crate::types::Context;

/// Sends a link to a GitHub repository.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn github(
	ctx: Context<'_>,
	#[description = "GitHub user."] user: String,
	#[description = "GitHub repository name."] repository: String,
) -> Result<()> {
	let response = format!(
		"[{user}/{repository}](https://github.com/{user}/{repository})",
	);

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.content(response);

	ctx.send(reply).await?;
	Ok(())
}
