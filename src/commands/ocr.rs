use anyhow::Result;
use caith::{RollResultType, Roller};
use poise::{
	serenity_prelude::{Attachment, CreateAllowedMentions, CreateAttachment},
	CreateReply,
};

use crate::types::Context;

/// Runs OCR on valid attachments.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn roll(
	ctx: Context<'_>,
	#[description = "The expression to roll."] text: Attachment,
) -> Result<()> {
	let ephemeral = ephemeral.unwrap_or(false);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	ctx.send(reply).await?;
	Ok(())
}
