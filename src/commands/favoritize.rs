use std::{env, fs, process};

use crate::{
	helpers::safe_delete,
	os_command::run_os_command,
	types::{ApplicationContext, Context},
};
use anyhow::{anyhow, Result};
use poise::{
	serenity_prelude::{
		Attachment, CreateAllowedMentions, CreateAttachment, EmbedThumbnail,
		Message,
	},
	CreateReply, Modal,
};

#[derive(Debug, Modal)]
struct FavoritizeModal {
	#[placeholder = "The attachment to use. (default: 0)"]
	attachment_index: Option<String>,
	#[placeholder = "Whether or not to show the message."]
	ephemeral: Option<String>,
}

#[derive(Debug)]
pub enum AttachmentOrThumbnail {
	Attachment(Attachment),
	Embed(EmbedThumbnail),
}
impl AttachmentOrThumbnail {
	async fn download(&self, client: &reqwest::Client) -> Result<Vec<u8>> {
		match self {
			AttachmentOrThumbnail::Attachment(a) => Ok(a.download().await?),
			AttachmentOrThumbnail::Embed(e) => {
				// Download the image from the proxy URL.
				let url = e.proxy_url.as_ref().ok_or_else(|| {
					anyhow!("Embed thumbnail has no proxy URL")
				})?;
				let request = client.get(url).send().await?;
				Ok(request.bytes().await?.to_vec())
			}
		}
	}

	fn filename(&self) -> String {
		match self {
			AttachmentOrThumbnail::Attachment(a) => a.filename.clone(),
			AttachmentOrThumbnail::Embed(e) => {
				// Parse the URL to get the filename.
				let url = &e.proxy_url;
				let url = if let Some(url) = url {
					url::Url::parse(&url).unwrap_or_else(|_| {
						url::Url::parse("https://example.com/thumbnail.png")
							.unwrap()
					})
				} else {
					return "thumbnail.png".to_string();
				};
				if let Some(path_segments) = url.path_segments() {
					let filename = path_segments
						.last()
						.unwrap_or_else(|| "thumbnail.png");
					filename.to_string()
				} else {
					"thumbnail.png".to_string()
				}
			}
		}
	}
}

/// Converts any image type into a 2 frame WebP so that it can be favorited on Discord.
#[poise::command(
	context_menu_command = "Favoritize Image",
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn favoritize_context_menu(
	ctx: ApplicationContext<'_>,
	#[description = "The message to turn into a favoritable image."]
	message: Message,
) -> Result<()> {
	let data = FavoritizeModal::execute(ctx)
		.await?
		.ok_or_else(|| anyhow!("No modal data."))?;

	let ephemeral = match data.ephemeral.as_deref() {
		Some("false") => false,
		Some(_) => true,
		None => false,
	};

	let attachments: Vec<AttachmentOrThumbnail> = message
		.attachments
		.iter()
		.map(|a| AttachmentOrThumbnail::Attachment(a.clone()))
		.chain(message.embeds.iter().filter_map(|e| {
			if let Some(thumbnail) = &e.thumbnail {
				if thumbnail.proxy_url.is_some() {
					Some(AttachmentOrThumbnail::Embed(thumbnail.clone()))
				} else {
					None
				}
			} else {
				None
			}
		}))
		.collect();

	// Get the attachment to turn into a favoritable image.
	let attachment_index = match data.attachment_index.as_deref() {
		Some(attachment_index) => attachment_index.parse::<usize>()?,
		None => 0,
	};
	let attachment = attachments.get(attachment_index).ok_or_else(|| {
		anyhow!(
			"You chose attachment {} but there {} only {} attachment{}.",
			attachment_index + 1,
			if attachments.len() == 1 { "is" } else { "are" },
			attachments.len(),
			if attachments.len() == 1 { "" } else { "s" },
		)
	})?;

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	let (new_image_data, new_image_name) = convert_to_animated_webp(
		&ctx.data().http,
		attachment,
		&attachment.filename(),
	)
	.await?;
	reply = reply
		.attachment(CreateAttachment::bytes(new_image_data, new_image_name));

	ctx.send(reply).await?;

	Ok(())
}

/// Converts any image type into a 2 frame WebP so that it can be favorited on Discord.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn favoritize(
	ctx: Context<'_>,
	#[description = "The image to turn into a favoritable image."]
	attachment: Attachment,
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

	let attachment = AttachmentOrThumbnail::Attachment(attachment);
	let (new_image_data, new_image_name) = convert_to_animated_webp(
		&ctx.data().http,
		&attachment,
		&attachment.filename(),
	)
	.await?;
	reply = reply
		.attachment(CreateAttachment::bytes(new_image_data, new_image_name));

	ctx.send(reply).await?;

	Ok(())
}

pub async fn convert_to_animated_webp(
	client: &reqwest::Client,
	attachment: &AttachmentOrThumbnail,
	attachment_name: &str,
) -> Result<(Vec<u8>, String)> {
	let image_path_template = env::temp_dir();
	let image_input = image_path_template.join(attachment_name);
	let mut image_output = image_path_template.join(attachment_name);
	image_output.set_extension("webp");

	fs::write(&image_input, attachment.download(&client).await?)?;

	// img2webp -near_lossless 100 -sharp_yuv -v -loop 0 input.png -d 1 -lossless -q 100 -m 6 -o output.webp

	// First, convert the image to a WebP with the best quality possible.
	let mut img2webp_command = process::Command::new("img2webp");
	img2webp_command.args([
		"-v",
		"-sharp_yuv",
		"-loop",
		"0",
		image_input.to_str().unwrap(),
		"-d",
		"1",
		"-lossless",
		"-q",
		"100",
		"-m",
		"6",
		"-o",
		image_output.to_str().unwrap(),
	]);
	let img2webp_output = run_os_command("img2webp", img2webp_command)?;

	if !img2webp_output.status.success() {
		// Delete the files.
		safe_delete(&image_input)?;
		safe_delete(&image_output)?;

		return Err(anyhow!(
			"```\n{}\n```",
			String::from_utf8_lossy(&img2webp_output.stderr)
		));
	}

	// webpmux -frame output.webp +0+0+0+1 -frame output.webp +0+0+0+1 -loop 0 -o output.webp

	// Then convert that one WebP into another WebP with two duplicate frames.
	// This should overwrite itself.
	// The -loop 1 flag is important to prevent the WebP from looping infinitely.
	let mut webpmux_command = process::Command::new("webpmux");
	webpmux_command.args([
		"-frame",
		image_output.to_str().unwrap(),
		"+0+0+0+1",
		"-frame",
		image_output.to_str().unwrap(),
		"+0+0+0+1",
		"-loop",
		"1",
		"-o",
		image_output.to_str().unwrap(),
	]);
	let webpmux_output = run_os_command("webpmux", webpmux_command)?;

	if webpmux_output.status.success() {
		let data = fs::read(&image_output)?;

		// Delete the files.
		safe_delete(&image_input)?;
		safe_delete(&image_output)?;

		Ok((
			data,
			image_output
				.file_name()
				.unwrap()
				.to_string_lossy()
				.to_string(),
		))
	} else {
		// Delete the files.
		safe_delete(&image_input)?;
		safe_delete(&image_output)?;

		Err(anyhow!(
			"```\n{}\n```",
			String::from_utf8_lossy(&webpmux_output.stderr)
		))
	}
}
