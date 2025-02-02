use anyhow::{anyhow, Result};
use chrono::{Datelike, Local, Timelike};
use poise::{
	serenity_prelude::{
		self as serenity, async_trait, CreateAllowedMentions,
	},
	ChoiceParameter, CreateReply, SlashArgError, SlashArgument,
};
use serde::{Deserialize, Serialize};

use crate::types::Context;

#[derive(Debug, Serialize, Deserialize, Clone, ChoiceParameter)]
enum Month {
	January,
	February,
	March,
	April,
	May,
	June,
	July,
	August,
	September,
	October,
	November,
	December,
}
impl Month {
	fn to_number(&self) -> u32 {
		match self {
			Self::January => 1,
			Self::February => 2,
			Self::March => 3,
			Self::April => 4,
			Self::May => 5,
			Self::June => 6,
			Self::July => 7,
			Self::August => 8,
			Self::September => 9,
			Self::October => 10,
			Self::November => 11,
			Self::December => 12,
		}
	}
}

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

struct InputTime {
	year: Option<i32>,
	month: Option<u32>,
	day: Option<u32>,
	hour: Option<u32>,
	minute: Option<u32>,
	second: Option<u32>,
}
impl InputTime {
	fn now() -> Self {
		let now = Local::now();
		Self {
			year: Some(now.year()),
			month: Some(now.month()),
			day: Some(now.day()),
			hour: Some(now.hour()),
			minute: Some(now.minute()),
			second: Some(now.second()),
		}
	}
	fn to_timestamp(&self) -> Result<i64> {
		// Get the current time
		let mut datetime = Local::now();

		// Determine year
		datetime = datetime
			.with_year(match self.year {
				Some(x) => x,
				None if self.second.is_some()
					|| self.minute.is_some()
					|| self.hour.is_some()
					|| self.day.is_some()
					|| self.month.is_some() =>
				{
					datetime.year()
				}
				None => 0,
			})
			.ok_or(anyhow!(""))?;

		// Determine month
		datetime = datetime
			.with_month(match self.month {
				Some(x) => x,
				None if self.second.is_some()
					|| self.minute.is_some()
					|| self.hour.is_some()
					|| self.day.is_some() =>
				{
					datetime.month()
				}
				None => 1,
			})
			.ok_or(anyhow!(""))?;

		// Determine day
		datetime = datetime
			.with_day(match self.day {
				Some(x) => x,
				None if self.second.is_some()
					|| self.minute.is_some()
					|| self.hour.is_some() =>
				{
					datetime.day()
				}
				None => 1,
			})
			.ok_or(anyhow!(""))?;

		// Determine hour
		datetime = datetime
			.with_hour(match self.hour {
				Some(x) => x,
				None if self.second.is_some() || self.minute.is_some() => {
					datetime.hour()
				}
				None => 0,
			})
			.ok_or(anyhow!(""))?;

		// Determine minute
		datetime = datetime
			.with_minute(match self.minute {
				Some(x) => x,
				None if self.second.is_some() => datetime.minute(),
				None => 0,
			})
			.ok_or(anyhow!(""))?;

		// Determine second
		datetime = datetime
			.with_second(self.second.unwrap_or(0))
			.ok_or(anyhow!(""))?; // Default to 0 for second

		Ok(datetime.timestamp())
	}
}

enum IdOrTime {
	Id(String),
	Time(i64),
	None,
}

/// Lets you easily create a timestamp from an ID or a datetime.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	aliases("timestamp"),
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn snowstamp(
	ctx: Context<'_>,
	#[description = "Discord snowflake ID."] id: Option<String>,
	#[description = "The year in the date."] year: Option<i32>,
	#[description = "The month in the date."] month: Option<Month>,
	#[description = "The day in the date."] day: Option<u32>,
	#[description = "The hour in the time."] hour: Option<u32>,
	#[description = "The minute in the time."] minute: Option<u32>,
	#[description = "The second in the time."] second: Option<u32>,
	#[description = "Timestamp format."] format: Option<TimestampFormat>,
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

	let format_letter = serde_plain::to_string::<TimestampFormat>(
		&format.unwrap_or(TimestampFormat::ShortDateTime),
	)?;

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	let id_or_time = match (id, year, month, day, hour, minute, second) {
		(Some(id), None, None, None, None, None, None) => IdOrTime::Id(id),
		(None, None, None, None, None, None, None) => {
			IdOrTime::Time(InputTime::now().to_timestamp()?)
		}
		(None, year, month, day, hour, minute, second) => IdOrTime::Time(
			InputTime {
				year,
				month: month.map(|m| m.to_number()),
				day,
				hour,
				minute,
				second,
			}
			.to_timestamp()?,
		),
		_ => IdOrTime::None,
	};

	match id_or_time {
		IdOrTime::Id(id) => {
			let response = format!(
				"<t:{}:{}>",
				serenity::UserId::new(id.parse::<u64>()?)
					.created_at()
					.timestamp(),
				format_letter
			);

			reply = reply.content(response);

			ctx.send(reply).await?;
			Ok(())
		}
		IdOrTime::Time(time) => {
			let response = format!("<t:{}:{}>", time, format_letter);

			reply = reply.content(response);

			ctx.send(reply).await?;
			Ok(())
		}
		IdOrTime::None => {
			let reply = CreateReply::default()
				.allowed_mentions(CreateAllowedMentions::default())
				.content("You must specify either a Discord snowflake ID or any combination of time values.")
				.ephemeral(true);
			ctx.send(reply).await?;
			Ok(())
		}
	}
}
