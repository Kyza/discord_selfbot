use std::collections::HashMap;

use anyhow::Result;
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use inline_format::format;
use poise::{
	serenity_prelude::{Attachment, CreateAllowedMentions},
	CreateReply,
};
use rusty_tesseract::Image;

use crate::types::Context;

pub async fn run_ocr(image: Attachment, languages: String) -> Result<String> {
	let image = image.download().await?;
	let image = image::load_from_memory(&image)?;

	let (width, height) = image.dimensions();
	let scale = 3;
	let image = DynamicImage::ImageLuma8(image.clone().into()).resize(
		width * scale,
		height * scale,
		FilterType::Lanczos3,
	);

	image.save_with_format("ocr.png", image::ImageFormat::Png)?;

	let image = Image::from_dynamic_image(&image).unwrap();

	// define parameters
	let tesseract_args = rusty_tesseract::Args {
		lang: languages,
		config_variables: HashMap::from([]),
		dpi: None,
		psm: None,
		oem: None,
	};

	// string output
	Ok(rusty_tesseract::image_to_string(&image, &tesseract_args)?)
}

// TODO: Add a context menu command to run OCR on a message's attachments.

/// Runs OCR on valid attachments.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn ocr(
	ctx: Context<'_>,
	#[description = "The image to run OCR on."] image: Attachment,
	#[description = "The languages to use for OCR."] languages: Option<
		String,
	>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let languages = languages.unwrap_or("eng".to_string());
	let ephemeral = ephemeral.unwrap_or(false);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	let output = run_ocr(image, languages).await?;

	ctx.send(reply.content(format!("```\n", output, "\n```")))
		.await?;
	Ok(())
}
