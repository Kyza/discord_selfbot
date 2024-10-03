use anyhow::Result;
use byte_unit::{Byte, Unit};
use num_format::ToFormattedString;
use poise::{
	serenity_prelude::{
		self as serenity, async_trait, CommandInteraction, CreateActionRow,
		CreateAllowedMentions, CreateAttachment, CreateButton, CreateEmbed,
		Mentionable, ResolvedValue,
	},
	CommandParameterChoice, CreateReply, SlashArgError, SlashArgument,
	SlashArgumentHack,
};
use reqwest::header;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
	fs::File, future::Future, io::Write, os::windows::io::AsHandle, pin::Pin,
	str::FromStr,
};

use crate::types::Context;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CobaltRequest {
	url: String,
	always_proxy: bool,
	filename_style: String,
	video_quality: CobaltVideoQuality,
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
	status: CobaltStatus,
	// Error
	error: Option<CobaltError>,
	// Tunnel / Redirect
	url: Option<String>,
	filename: Option<String>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct CobaltError {
	code: String,
	context: Option<CobaltErrorContext>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct CobaltErrorContext {
	service: String,
	limit: i32,
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
		.post("https://api.cobalt.tools/")
		.header(header::ACCEPT, "application/json")
		.header(header::CONTENT_TYPE, "application/json")
		.json(&CobaltRequest {
			url: url.to_string(),
			always_proxy: true,
			filename_style: "nerdy".to_string(),
			video_quality: quality.unwrap_or(CobaltVideoQuality::Quality720),
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
					.content(format!(
						"{:?} `{:?}`",
						response.status, response.filename
					));
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

				println!("File size: {:#}", Byte::from_u64(content_length));

				let reply = CreateReply::default()
					.allowed_mentions(CreateAllowedMentions::default())
					.attachment(CreateAttachment::bytes(
						content,
						response
							.filename
							.clone()
							.expect("There was no filename."),
					))
					.components(vec![CreateActionRow::Buttons(vec![
						CreateButton::new_link(response.url.clone().unwrap())
							.label("Download"),
					])])
					.content(format!("<{}>", url.to_string()));
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
							"Something went wrong.```json\n{}\n```File size: `{:#}`",
							e,
							Byte::from_u64(content_length)
						));
					ctx.send(reply).await?;
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
