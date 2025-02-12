use anyhow::Result;
use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

use crate::config::Context;

/// Sends a formatted link to a GitHub profile or repository.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn github(
	ctx: Context<'_>,
	#[description = "GitHub user name."] user: String,
	#[description = "GitHub repository name."] repository: Option<String>,
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

	let response = if let Some(repository) = repository {
		format!(
			"[{user}/{repository}](https://github.com/{user}/{repository})",
		)
	} else {
		format!("[{user}](https://github.com/{user})",)
	};

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.content(response)
		.ephemeral(ephemeral);

	ctx.send(reply).await?;
	Ok(())
}
