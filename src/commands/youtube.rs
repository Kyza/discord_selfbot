use std::{env, fs, process};

use crate::{os_command::run_os_command, types::Context};
use anyhow::{anyhow, Result};
use poise::{
	serenity_prelude::{CreateAllowedMentions, CreateAttachment},
	CreateReply,
};
use rand::Rng;
use url::Url;

const FORMAT: &str = "mp4";

/// [Experimental] Downloads a video from YouTube and sends it.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn youtube(
	ctx: Context<'_>,
	#[description = "YouTube URL."] url: Url,
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

	// Generate a random filename.
	let yt_dlp_output_path_template = &env::temp_dir()
		.join(rand::thread_rng().gen::<u64>().to_string())
		.to_string_lossy()
		.to_string();
	let yt_dlp_output_final_path =
		&format!("{yt_dlp_output_path_template}.{FORMAT}");

	println!("{:?}", yt_dlp_output_path_template);
	let mut yt_dlp_command = process::Command::new("yt-dlp");
	yt_dlp_command.args([
		url.as_str(),
		"-o",
		yt_dlp_output_path_template,
		"--recode-video",
		FORMAT,
	]);
	let yt_dlp_output = run_os_command(yt_dlp_command)?;

	if yt_dlp_output.status.success() {
		// Read the file.
		println!("{}", yt_dlp_output_final_path);
		let data = fs::read(yt_dlp_output_final_path)?;
		// Attach the file.
		reply = reply.attachment(CreateAttachment::bytes(
			data,
			format!("video.{FORMAT}"),
		));
	} else {
		// Delete the file.
		if fs::exists(yt_dlp_output_final_path)? {
			fs::remove_file(yt_dlp_output_final_path)?;
		}

		return Err(anyhow!(
			"```\n{}\n```",
			String::from_utf8_lossy(&yt_dlp_output.stderr)
		));
	}

	ctx.send(reply).await?;

	// Delete the file.
	if fs::exists(yt_dlp_output_final_path)? {
		fs::remove_file(yt_dlp_output_final_path)?;
	}

	Ok(())
}
