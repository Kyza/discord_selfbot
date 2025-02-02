use anyhow::Result;
use inline_format::format;
use poise::{
	serenity_prelude::{
		Attachment, Colour, CreateAllowedMentions, CreateAttachment, Embed,
		EmbedImage, EmbedThumbnail, Timestamp,
	},
	CreateReply,
};

use crate::{helpers::easy_set_file_name, types::Context};

/// Builds an embed and sends it.
/// RON representation can be used to send multiple embeds.
///
/// https://github.com/ron-rs/ron
/// https://docs.rs/poise/latest/poise/serenity_prelude/struct.Embed.html
///
/// Pay attention. Some of the fields are renamed such as `colour` to `color`.
/// Check the docs for the correct field names.
///
/// You can use 0xRRGGBB for colors. Setting it to None will use the default color from the config.
///
/// More complex settings require the usage of RON.
///
/// The `video` field can't be set using Discord's bot APIs.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn embed(
	ctx: Context<'_>,
	#[description = "Embed RON."] ron: Option<String>,

	#[description = "Embed title."] title: Option<String>,
	#[description = "Embed kind."] kind: Option<String>,
	#[description = "Embed description."] description: Option<String>,
	#[description = "Embed URL."] url: Option<String>,
	#[description = "Embed timestamp."] timestamp: Option<Timestamp>,
	#[description = "Embed color. (0xRRGGBB)"] color: Option<String>,
	#[description = "Embed footer. (RON)"] footer: Option<String>,
	#[description = "Embed image."] image: Option<Attachment>,
	#[description = "Embed thumbnail."] thumbnail: Option<Attachment>,
	#[description = "Embed provider. (RON)"] provider: Option<String>,
	#[description = "Embed author. (RON)"] author: Option<String>,
	#[description = "Embed fields. (RON)"] fields: Option<String>,

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

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	let embeds = {
		if let Some(ron) = ron {
			// Try a Vec and a single embed.
			if let Ok(embeds) = ron::from_str::<Vec<Embed>>(&ron.clone()) {
				embeds
			} else {
				vec![ron::from_str::<Embed>(&ron)?]
			}
		} else {
			// Parse the fields into an embed.
			// Be as user-friendly here as possible without losing functionality.
			let mut embed = Embed::default();
			embed.title = title;
			embed.kind = kind;
			embed.description = description;
			embed.url = url;
			embed.timestamp = timestamp;
			if let Some(color) = color {
				embed.colour = Some(Colour::new(ron::from_str(&color)?));
			} else {
				embed.colour =
					Some(ctx.data().config.embed_color.clone().into());
			}
			if let Some(footer) = footer {
				embed.footer = Some(ron::from_str(&footer)?);
			}
			if let Some(attachment) = image {
				let mut downloaded_attachment =
					CreateAttachment::url(ctx.http(), &attachment.url)
						.await?;
				downloaded_attachment.filename =
					easy_set_file_name(&attachment.filename, "image")
						.to_string();
				downloaded_attachment.description = attachment.description;
				reply = reply.attachment(downloaded_attachment.clone());
				let mut image = ron::from_str::<EmbedImage>("(url:\"\")")?;
				image.url =
					format!("attachment://", downloaded_attachment.filename);
				image.width = attachment.width;
				image.height = attachment.height;
				embed.image = Some(image);
			}
			if let Some(attachment) = thumbnail {
				let mut downloaded_attachment =
					CreateAttachment::url(ctx.http(), &attachment.url)
						.await?;
				downloaded_attachment.filename =
					easy_set_file_name(&attachment.filename, "thumbnail")
						.to_string();
				downloaded_attachment.description = attachment.description;
				reply = reply.attachment(downloaded_attachment.clone());
				let mut image =
					ron::from_str::<EmbedThumbnail>("(url:\"\")")?;
				image.url =
					format!("attachment://", downloaded_attachment.filename);
				image.width = image.width;
				image.height = image.height;
				embed.thumbnail = Some(image);
			}
			if let Some(provider) = provider {
				embed.provider = Some(ron::from_str(&provider)?);
			}
			if let Some(author) = author {
				embed.author = Some(ron::from_str(&author)?);
			}
			if let Some(fields) = fields {
				embed.fields = ron::from_str(&fields)?;
			}
			vec![embed]
		}
	};

	for mut embed in embeds {
		embed.colour = embed
			.colour
			.or(Some(ctx.data().config.embed_color.clone().into()));
		reply = reply.embed(embed.try_into()?);
	}

	ctx.send(reply).await?;
	Ok(())
}
