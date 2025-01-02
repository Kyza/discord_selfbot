use std::{env, fs, process};

use crate::{
	helpers::{safe_delete, AttachmentOrThumbnail},
	os_command::run_os_command,
	types::{ApplicationContext, Context},
};
use anyhow::{anyhow, Result};
use poise::{
	serenity_prelude::{
		Attachment, CreateAllowedMentions, CreateAttachment, Message,
	},
	CreateReply, Modal,
};

#[derive(Debug, Modal)]
#[name = "Convert To JXL"]
struct JXLModal {
	#[name = "Attachment Index"]
	#[placeholder = "The index of the attachment to use. (default: 0)"]
	attachment_index: Option<String>,
	#[placeholder = "Whether or not to show the message."]
	ephemeral: Option<String>,
}

/// Converts an image to JXL.
#[poise::command(
	context_menu_command = "Convert To JXL",
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn jxl_context_menu(
	ctx: ApplicationContext<'_>,
	#[description = "The message to convert to JXL."] message: Message,
) -> Result<()> {
	let data = JXLModal::execute(ctx)
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

	// Get the attachment to convert to JXL.
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

	let (new_image_data, new_image_name) = convert_to_animated_jxl(
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

/// Converts an image to JXL.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn jxl(
	ctx: Context<'_>,
	#[description = "The image to convert to JXL."] attachment: Attachment,
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
	let (new_image_data, new_image_name) = convert_to_animated_jxl(
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

pub async fn convert_to_animated_jxl(
	client: &reqwest::Client,
	attachment: &AttachmentOrThumbnail,
	attachment_name: &str,
) -> Result<(Vec<u8>, String)> {
	let image_path_template = env::temp_dir();
	let image_input = image_path_template.join(attachment_name);
	let mut image_output = image_path_template.join(attachment_name);
	image_output.set_extension("jxl");

	fs::write(&image_input, attachment.download(&client).await?)?;

	let output = {
		let mut cjxl_command = process::Command::new("cjxl");
		cjxl_command.args([
			"-v",
			image_input.to_str().unwrap(),
			"-e",
			"10",
			image_output.to_str().unwrap(),
		]);
		let cjxl_output = run_os_command("cjxl", cjxl_command)?;

		cjxl_output
	};

	if !output.status.success() {
		// Delete the files.
		safe_delete(&image_input)?;
		safe_delete(&image_output)?;

		return Err(anyhow!(
			"```\n{}\n```",
			String::from_utf8_lossy(&output.stderr)
		));
	}

	let data = fs::read(&image_output)?;

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
}
