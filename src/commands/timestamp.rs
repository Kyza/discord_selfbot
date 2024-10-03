use anyhow::Result;
use poise::{
	serenity_prelude::{
		self as serenity, async_trait, CreateAllowedMentions,
		CreateAttachment, Mentionable,
	},
	CreateReply, SlashArgError, SlashArgument,
};
use serde::{Deserialize, Serialize};

use crate::types::Context;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TimestampFormat {
	#[serde(rename = "t")]
	ShortTime,
	#[serde(rename = "T")]
	LongTime,
	#[serde(rename = "d")]
	ShortDate,
	#[serde(rename = "D")]
	LongDate,
	#[serde(rename = "f")]
	ShortDateTime,
	#[serde(rename = "F")]
	LongDateTime,
	#[serde(rename = "R")]
	RelativeTime,
}
#[async_trait]
impl SlashArgument for TimestampFormat {
	async fn extract(
		_: &serenity::Context,
		_: &serenity::CommandInteraction,
		value: &serenity::ResolvedValue<'_>,
	) -> Result<TimestampFormat, SlashArgError> {
		match *value {
			serenity::ResolvedValue::String(x) => {
				serde_plain::from_str::<TimestampFormat>(x).map_err(|_| {
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
			.add_string_choice("Short Time", "t")
			.add_string_choice("Long Time", "T")
			.add_string_choice("Short Date", "d")
			.add_string_choice("Long Date", "D")
			.add_string_choice("Short DateTime", "f")
			.add_string_choice("Long DateTime", "F")
			.add_string_choice("Relative Time", "R")
	}
}

/// Tells you when an account was created.
/// Requires either a user ID or a timestamp.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn timestamp(
	ctx: Context<'_>,
	#[description = "Discord snowflake ID."] id: Option<String>,
	#[description = "Timestamp parsable by `dateparser::parse`."]
	timestamp: Option<String>,
	#[description = "Timestamp format."] format: Option<TimestampFormat>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	if id.is_none() && timestamp.is_none() {
		let reply = CreateReply::default()
			.allowed_mentions(CreateAllowedMentions::default())
			.content("You must specify either a Discord snowflake ID or a timestamp.")
			.ephemeral(true);
		ctx.send(reply).await?;
		return Ok(());
	}

	if id.is_some() && timestamp.is_some() {
		let reply = CreateReply::default()
			.allowed_mentions(CreateAllowedMentions::default())
			.content("You cannot specify both a Discord snowflake ID and a timestamp.")
			.ephemeral(true);
		ctx.send(reply).await?;
		return Ok(());
	}

	let format_letter = serde_plain::to_string::<TimestampFormat>(
		&format.unwrap_or(TimestampFormat::ShortDateTime),
	)?;

	if let Some(id) = id {
		let response = format!(
			"<t:{}:{}>",
			serenity::UserId::new(id.parse::<u64>()?)
				.created_at()
				.timestamp(),
			format_letter
		);

		let reply = CreateReply::default()
			.allowed_mentions(CreateAllowedMentions::default())
			.content(response)
			.ephemeral(ephemeral.unwrap_or(true));

		ctx.send(reply).await?;
	}

	if let Some(timestamp) = timestamp {
		let response = format!(
			"<t:{}:{}>",
			dateparser::parse(&timestamp)?.timestamp(),
			format_letter
		);

		let reply = CreateReply::default()
			.allowed_mentions(CreateAllowedMentions::default())
			.content(response)
			.ephemeral(ephemeral.unwrap_or(true));

		ctx.send(reply).await?;
	}

	Ok(())
}
