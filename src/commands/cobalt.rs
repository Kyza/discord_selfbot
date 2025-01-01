use std::io::Write;

use anyhow::Result;
use byte_unit::Byte;
use poise::{
	serenity_prelude::{
		self as serenity, async_trait, CreateActionRow,
		CreateAllowedMentions, CreateAttachment, CreateButton,
	},
	CreateReply, SlashArgError, SlashArgument,
};
use reqwest::header;
use serde::{Deserialize, Serialize};

use crate::{helpers::change_extension, media, types::Context};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CobaltRequest {
	url: String,
	always_proxy: bool,
	filename_style: String,
	video_quality: CobaltVideoQuality,
	twitter_gif: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CobaltVideoQuality {
	#[serde(rename = "144")]
	Quality144,
	#[serde(rename = "240")]
	Quality240,
	#[serde(rename = "360")]
	Quality360,
	#[serde(rename = "480")]
	Quality480,
	#[serde(rename = "720")]
	Quality720,
	#[serde(rename = "1080")]
	Quality1080,
	#[serde(rename = "1440")]
	Quality1440,
	#[serde(rename = "2160")]
	Quality2160,
	#[serde(rename = "max")]
	QualityMax,
}
#[async_trait]
impl SlashArgument for CobaltVideoQuality {
	async fn extract(
		_: &serenity::Context,
		_: &serenity::CommandInteraction,
		value: &serenity::ResolvedValue<'_>,
	) -> Result<CobaltVideoQuality, SlashArgError> {
		match *value {
			serenity::ResolvedValue::String(x) => {
				serde_plain::from_str::<CobaltVideoQuality>(x).map_err(|_| {
					SlashArgError::new_command_structure_mismatch(
						"received invalid quality",
					)
				})
			}
			_ => Err(SlashArgError::new_command_structure_mismatch(
				"expected string",
			)),
		}
	}

	fn create(
		builder: serenity::CreateCommandOption,
	) -> serenity::CreateCommandOption {
		builder
			.add_string_choice("144p", "144")
			.add_string_choice("240p", "240")
			.add_string_choice("360p", "360")
			.add_string_choice("480p", "480")
			.add_string_choice("720p", "720")
			.add_string_choice("1080p", "1080")
			.add_string_choice("1440p", "1440")
			.add_string_choice("2160p", "2160")
			.add_string_choice("Max", "max")
	}
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CobaltStatus {
	Error,
	Picker,
	Redirect,
	Tunnel,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CobaltResponse {
	pub status: CobaltStatus,
	// Error
	pub error: Option<CobaltError>,
	// Tunnel / Redirect
	pub url: Option<String>,
	pub filename: Option<String>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct CobaltError {
	pub code: String,
	pub context: Option<CobaltErrorContext>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct CobaltErrorContext {
	pub service: String,
	pub limit: i32,
}

/// Downloads media from a URL using the Cobalt API and sends it.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn cobalt(
	ctx: Context<'_>,
	#[description = "Media URL."] url: url::Url,
	#[description = "Video resolution. Default 720p."]
	#[lazy]
	resolution: Option<CobaltVideoQuality>,
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

	let result = ctx
		.data()
		.http
		.post("https://localhost:9000/")
		.header(header::ACCEPT, "application/json")
		.header(header::CONTENT_TYPE, "application/json")
		.json(&CobaltRequest {
			url: url.to_string(),
			always_proxy: true,
			filename_style: "nerdy".to_string(),
			video_quality: resolution
				.unwrap_or(CobaltVideoQuality::Quality720),
			twitter_gif: true,
		})
		.send()
		.await?
		.text()
		.await?;

	if let Ok(response) = serde_json::from_str::<CobaltResponse>(&result) {
		match response.status {
			CobaltStatus::Error => {
				reply = reply.content(format!("```\n{:#?}\n```", response));
				ctx.send(reply).await?;
			}
			CobaltStatus::Picker => {
				reply = reply.content(
					"Picker was reached and I haven't made that yet.",
				);
				ctx.send(reply).await?;
			}
			CobaltStatus::Redirect | CobaltStatus::Tunnel => {
				// Download the file.
				let res = ctx
					.data()
					.http
					.get(
						response
							.url
							.clone()
							.expect("There was no download URL."),
					)
					.send()
					.await?;

				let content = res.bytes().await?;
				let content_length = content.len() as u64;

				let downloaded_file_name = response
					.filename
					.clone()
					.expect("There was no filename.");

				let is_too_large = |size| size > 8 * 1024 * 1024;

				println!(
					"File name before: {}",
					downloaded_file_name.clone()
				);
				println!(
					"File size before: {:#}",
					Byte::from_u64(content_length)
				);

				// Save the file.
				// Not tempfile.
				let mut file = tempfile::NamedTempFile::new()?;
				file.write_all(&content)?;
				// std::fs::copy(file.path(), downloaded_file_name.clone())?;
				// Compress the file.
				let (compressed_file, extension) =
					media::compress_file(file.path())?;
				let upload_file_name = &change_extension(
					downloaded_file_name.clone(),
					&extension,
				)
				.to_string_lossy()
				.to_string();
				// Save to current directory.
				// std::fs::copy(
				// 	compressed_file.path(),
				// 	upload_file_name.clone(),
				// )?;

				let compressed_file_size =
					compressed_file.as_file().metadata()?.len();

				println!("File name after: {}", upload_file_name);

				println!(
					"File size after: {:#}",
					Byte::from_u64(compressed_file_size)
				);

				// If the file is larger than 8MB, send a warning.
				if is_too_large(compressed_file_size) {
					reply = reply
						.components(vec![CreateActionRow::Buttons(vec![
							CreateButton::new_link(response.url.unwrap())
								.label("Download"),
						])])
						.content(format!(
							"<{}>\n-# {}\nFile size: `{:#}`",
							url,
							upload_file_name,
							Byte::from_u64(compressed_file_size)
						));
					ctx.send(reply).await?;
					return Ok(());
				}

				// Read the file contents.
				let compressed_file_content =
					std::fs::read(compressed_file.path())?;

				reply = reply
					.attachment(CreateAttachment::bytes(
						compressed_file_content,
						upload_file_name,
					))
					.components(vec![CreateActionRow::Buttons(vec![
						CreateButton::new_link(response.url.clone().unwrap())
							.label("Download"),
					])])
					.content(format!("<{}>\n-# {}", url, upload_file_name,));
				let sent_response = ctx.send(reply.clone()).await;

				if let Err(e) = sent_response {
					println!("Error sending message: {}", e);
					reply = reply
						.components(vec![CreateActionRow::Buttons(vec![
							CreateButton::new_link(response.url.unwrap())
								.label("Download"),
						])])
						.content(format!(
							"<{}>\n-# {}\n```json\n{}\n```File size: `{:#}`",
							url,
							upload_file_name,
							e,
							Byte::from_u64(content_length)
						));
					ctx.send(reply).await?;

					drop(compressed_file);
				}
			}
		}
	} else {
		reply = reply.content(format!(
			"Something went wrong.```json\n{}\n```",
			result
		));
		ctx.send(reply).await?;
	}
	Ok(())
}
