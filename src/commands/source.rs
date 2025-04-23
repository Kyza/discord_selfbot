use anyhow::Result;
use poise::{
	serenity_prelude::{
		CreateActionRow, CreateAllowedMentions, CreateButton,
		CreateComponent, CreateContainer, MessageFlags,
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
		.flags(MessageFlags::IS_COMPONENTS_V2)
		.components(
			[CreateComponent::Container(
				CreateContainer::new(
					[CreateComponent::ActionRow(CreateActionRow::Buttons(
						[CreateButton::new_link(
							"https://github.com/Kyza/discord_selfbot",
						)
						.label("Source Code")]
						.to_vec()
						.into(),
					))]
					.to_vec()
					.to_owned(),
				)
				.accent_color(ctx.data().config.embed_color.clone()),
			)]
			.to_vec()
			.to_owned(),
		)
		.ephemeral(ephemeral);

	ctx.send(reply).await?;
	Ok(())
}
