use anyhow::Result;
use poise::{
	serenity_prelude::{
		self as serenity, CreateAllowedMentions, Mentionable,
	},
	CreateReply,
};

use crate::types::Context;

/// Sends the timestamp of the ID or user's creation date.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn age(
	ctx: Context<'_>,
	#[description = "Selected user."] user: Option<serenity::User>,
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

	let u = user.as_ref().unwrap_or_else(|| ctx.author());
	let response = format!(
		"{} was created at <t:{}:F>.",
		u.id.mention(),
		u.created_at().timestamp()
	);

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.content(response)
		.ephemeral(ephemeral);

	ctx.send(reply).await?;
	Ok(())
}
