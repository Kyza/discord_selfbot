use std::{io::Write, os::windows::fs::MetadataExt};

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

/// Downloads a media file from a URL and sends it.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn cobalt(
	ctx: Context<'_>,
	#[description = "Media URL."] url: url::Url,
	#[description = "Video quality. Default 720p."]
	#[lazy]
	quality: Option<CobaltVideoQuality>,
) -> Result<()> {
	ctx.defer().await?;

	let result = ctx
		.data()
		.http
		.post("https://nyc1.coapi.ggtyler.dev/")
		.header(header::ACCEPT, "application/json")
		.header(header::CONTENT_TYPE, "application/json")
		.json(&CobaltRequest {
			url: url.to_string(),
			always_proxy: true,
			filename_style: "nerdy".to_string(),
			video_quality: quality.unwrap_or(CobaltVideoQuality::Quality720),
			twitter_gif: true,
		})
		.send()
		.await?
		.text()
		.await?;

	if let Ok(response) = serde_json::from_str::<CobaltResponse>(&result) {
		match response.status {
			CobaltStatus::Error => {
				let reply = CreateReply::default()
					.allowed_mentions(CreateAllowedMentions::default())
					.content(format!("```\n{:#?}\n```", response));
				ctx.send(reply).await?;
			}
			CobaltStatus::Picker => {
				let reply = CreateReply::default()
					.allowed_mentions(CreateAllowedMentions::default())
					.content(
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
						&response
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
				let (compressed_file, extension) = media::convert_file(
					file.path(),
					is_too_large(content_length),
				)?;
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
					compressed_file.as_file().metadata()?.file_size();

				println!("File name after: {}", upload_file_name);

				println!(
					"File size after: {:#}",
					Byte::from_u64(compressed_file_size)
				);

				// If the file is larger than 8MB, send a warning.
				if is_too_large(compressed_file_size) {
					let reply = CreateReply::default()
						.allowed_mentions(CreateAllowedMentions::default())
						.components(vec![CreateActionRow::Buttons(vec![
							CreateButton::new_link(response.url.unwrap())
								.label("Download"),
						])])
						.content(format!(
							"<{}>\n-# {}\nFile size: `{:#}`",
							url.to_string(),
							upload_file_name,
							Byte::from_u64(compressed_file_size)
						));
					ctx.send(reply).await?;
					return Ok(());
				}

				// Read the file contents.
				let compressed_file_content =
					std::fs::read(compressed_file.path())?;

				let reply = CreateReply::default()
					.allowed_mentions(CreateAllowedMentions::default())
					.attachment(CreateAttachment::bytes(
						compressed_file_content,
						upload_file_name,
					))
					.components(vec![CreateActionRow::Buttons(vec![
						CreateButton::new_link(response.url.clone().unwrap())
							.label("Download"),
					])])
					.content(format!(
						"<{}>\n-# {}",
						url.to_string(),
						upload_file_name,
					));
				let sent_response = ctx.send(reply).await;

				if let Err(e) = sent_response {
					println!("Error sending message: {}", e);
					let reply = CreateReply::default()
						.allowed_mentions(CreateAllowedMentions::default())
						.components(vec![CreateActionRow::Buttons(vec![
							CreateButton::new_link(response.url.unwrap())
								.label("Download"),
						])])
						.content(format!(
							"<{}>\n-# {}\n```json\n{}\n```File size: `{:#}`",
							url.to_string(),
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
		let reply = CreateReply::default()
			.allowed_mentions(CreateAllowedMentions::default())
			.content(format!(
				"Something went wrong.```json\n{}\n```",
				result
			));
		ctx.send(reply).await?;
	}
	Ok(())
}
