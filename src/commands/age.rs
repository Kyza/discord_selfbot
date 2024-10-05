use anyhow::Result;
use poise::{
	serenity_prelude::{
		self as serenity, CreateAllowedMentions, Mentionable,
	},
	CreateReply,
};

use crate::types::Context;

/// Tells you when an account was created.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn age(
	ctx: Context<'_>,
	#[description = "Selected user."] user: Option<serenity::User>,
) -> Result<()> {
	let u = user.as_ref().unwrap_or_else(|| ctx.author());
	let response = format!(
		"{} was created at <t:{}:F>.",
		u.id.mention(),
		u.created_at().timestamp()
	);

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.content(response);

	ctx.send(reply).await?;
	Ok(())
}
