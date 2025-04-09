use anyhow::Result;
use poise::{
	serenity_prelude::{
		CreateActionRow, CreateAllowedMentions, CreateButton,
	},
	CreateReply,
};

use crate::config::Context;

/// Sends the link to the bot's source code.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn source(
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

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.components(vec![CreateActionRow::Buttons(vec![
			CreateButton::new_link("https://github.com/Kyza/discord_selfbot")
				.label("Source Code"),
		])])
		.ephemeral(ephemeral);

	ctx.send(reply).await?;
	Ok(())
}
