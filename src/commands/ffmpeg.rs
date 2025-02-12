use std::{env, fs, process};

use crate::{
	helpers::{safe_delete, AttachmentOrThumbnail},
	os_command::run_os_command,
	config::Context,
};
use anyhow::{anyhow, Result};
use inline_format::format;
use poise::{
	serenity_prelude::{Attachment, CreateAllowedMentions, CreateAttachment},
	CreateReply,
};
use rand::Rng;

/// Runs a basic FFmpeg command on uploaded media.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn ffmpeg(
	ctx: Context<'_>,
	#[description = "Attachment #1."] attachment_1: Attachment,
	#[description = "The FFmpeg flags to use."] flags: Option<String>,
	#[description = "The output file name."] output_name: String,
	#[description = "Attachment #2."] attachment_2: Option<Attachment>,
	#[description = "Attachment #3."] attachment_3: Option<Attachment>,
	#[description = "Attachment #4."] attachment_4: Option<Attachment>,
	#[description = "Attachment #5."] attachment_5: Option<Attachment>,
	#[description = "Attachment #6."] attachment_6: Option<Attachment>,
	#[description = "Attachment #7."] attachment_7: Option<Attachment>,
	#[description = "Attachment #8."] attachment_8: Option<Attachment>,
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

	let flags = flags.unwrap_or_default();

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	let attachments: Vec<AttachmentOrThumbnail> = vec![
		Some(AttachmentOrThumbnail::Attachment(attachment_1)),
		attachment_2.map(AttachmentOrThumbnail::Attachment),
		attachment_3.map(AttachmentOrThumbnail::Attachment),
		attachment_4.map(AttachmentOrThumbnail::Attachment),
		attachment_5.map(AttachmentOrThumbnail::Attachment),
		attachment_6.map(AttachmentOrThumbnail::Attachment),
		attachment_7.map(AttachmentOrThumbnail::Attachment),
		attachment_8.map(AttachmentOrThumbnail::Attachment),
	] // Filter out Nones.
	.into_iter()
	.flatten()
	.collect();
	let new_image_data =
		run_ffmpeg(&ctx, &attachments, &flags, &output_name).await?;
	reply = reply
		.attachment(CreateAttachment::bytes(new_image_data, output_name));

	ctx.send(reply).await?;

	Ok(())
}

pub async fn run_ffmpeg(
	ctx: &Context<'_>,
	attachments: &Vec<AttachmentOrThumbnail>,
	flags: &str,
	output_name: &str,
) -> Result<Vec<u8>> {
	let path_template = env::temp_dir();
	// Generate a unique name for each input file.
	let mut input_files = Vec::new();
	for attachment in attachments {
		let num: u32 = rand::thread_rng().gen();
		let attachment_name = format!(num, "_", attachment.filename());

		// Save each file.
		let input_file_path = path_template.join(attachment_name);
		input_files.push(input_file_path.clone());
		fs::write(
			&input_file_path,
			attachment.download(&ctx.data().http).await?,
		)?;
	}

	// Output file in temp dir.
	let output_file_path = path_template.join(output_name);

	let mut ffmpeg_command = process::Command::new("ffmpeg");

	for input_file in &input_files {
		ffmpeg_command.args(["-i", input_file.to_str().unwrap()]);
	}
	if !flags.is_empty() {
		ffmpeg_command.args(flags.split_whitespace());
	}
	ffmpeg_command.args([output_file_path.to_str().unwrap()]);

	let ffmpeg_command_output = run_os_command("ffmpeg", ffmpeg_command)?;

	if !ffmpeg_command_output.status.success() {
		// Delete the files.
		for input_file in input_files {
			safe_delete(&input_file)?;
		}
		safe_delete(&output_file_path)?;

		return Err(anyhow!(
			"```\n{}\n```",
			String::from_utf8_lossy(&ffmpeg_command_output.stderr)
		));
	}

	let data = fs::read(&output_file_path)?;

	// Delete the files.
	for input_file in input_files {
		safe_delete(&input_file)?;
	}
	safe_delete(&output_file_path)?;

	Ok(data)
}
