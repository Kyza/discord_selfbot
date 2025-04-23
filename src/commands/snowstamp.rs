use anyhow::{anyhow, Result};
use chrono::{Datelike, Timelike};
use inline_format::format;
use poise::{
	serenity_prelude::{
		self as serenity, async_trait, CreateAllowedMentions,
		CreateComponent, CreateContainer, CreateTextDisplay, MessageFlags,
	},
	ChoiceParameter, CreateReply, SlashArgError, SlashArgument,
};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::config::{Config, Context};

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

#[derive(
	Debug, Serialize, Deserialize, Clone, EnumIter, strum_macros::Display,
)]
pub enum TimestampFormat {
	#[serde(rename = "t")]
	#[strum(serialize = "Short Time")]
	ShortTime,
	#[serde(rename = "T")]
	#[strum(serialize = "Long Time")]
	LongTime,
	#[serde(rename = "d")]
	#[strum(serialize = "Short Date")]
	ShortDate,
	#[serde(rename = "D")]
	#[strum(serialize = "Long Date")]
	LongDate,
	#[serde(rename = "f")]
	#[strum(serialize = "Short Date Time")]
	ShortDateTime,
	#[serde(rename = "F")]
	#[strum(serialize = "Long Date Time")]
	LongDateTime,
	#[serde(rename = "R")]
	#[strum(serialize = "Relative Time")]
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
	fn now(config: &Config) -> Self {
		let now = config.timezone.local();
		Self {
			year: Some(now.year()),
			month: Some(now.month()),
			day: Some(now.day()),
			hour: Some(now.hour()),
			minute: Some(now.minute()),
			second: Some(now.second()),
		}
	}
	fn to_timestamp(&self, config: &Config) -> Result<i64> {
		// Get the current time.
		let mut datetime = config.timezone.local();

		// Determine year.
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

		// Determine month.
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

		// Determine day.
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

		// Determine hour.
		datetime = datetime
			.with_hour(match self.hour {
				Some(x) => x,
				None if self.second.is_some() || self.minute.is_some() => {
					datetime.hour()
				}
				None => 0,
			})
			.ok_or(anyhow!(""))?;

		// Determine minute.
		datetime = datetime
			.with_minute(match self.minute {
				Some(x) => x,
				None if self.second.is_some() => datetime.minute(),
				None => 0,
			})
			.ok_or(anyhow!(""))?;

		// Determine second.
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
	// #[description = "Timestamp format."] format: Option<TimestampFormat>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let ephemeral = ephemeral.unwrap_or(true);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.flags(MessageFlags::IS_COMPONENTS_V2)
		.ephemeral(ephemeral);

	let id_or_time = match (id, year, month, day, hour, minute, second) {
		(Some(id), None, None, None, None, None, None) => IdOrTime::Id(id),
		(None, None, None, None, None, None, None) => IdOrTime::Time(
			InputTime::now(&ctx.data().config)
				.to_timestamp(&ctx.data().config)?,
		),
		(None, year, month, day, hour, minute, second) => IdOrTime::Time(
			InputTime {
				year,
				month: month.map(|m| m.to_number()),
				day,
				hour,
				minute,
				second,
			}
			.to_timestamp(&ctx.data().config)?,
		),
		_ => IdOrTime::None,
	};

	let mut timestamps = Vec::new();

	let time = match id_or_time {
		IdOrTime::Id(id) => serenity::UserId::new(id.parse::<u64>()?)
			.created_at()
			.timestamp(),
		IdOrTime::Time(time) => time,
		IdOrTime::None => {
			let reply = CreateReply::default()
				.allowed_mentions(CreateAllowedMentions::default())
				.content("You must specify either a Discord snowflake ID or any combination of time values.")
				.ephemeral(true);
			ctx.send(reply).await?;
			return Ok(());
		}
	};

	for format in TimestampFormat::iter() {
		let format_letter =
			serde_plain::to_string::<TimestampFormat>(&format)?;
		let timestamp_string = format!("<t:", time, ":", format_letter, ">");
		timestamps.push(CreateComponent::TextDisplay(
			CreateTextDisplay::new(format!(
				"## ",
				format,
				"\n",
				timestamp_string,
				"\n```\n",
				timestamp_string,
				"\n```"
			)),
		));
	}

	let container = CreateComponent::Container(
		CreateContainer::new(timestamps)
			.accent_color(ctx.data().config.embed_color.clone()),
	);

	reply = reply.components(vec![container].to_owned());

	ctx.send(reply).await?;

	Ok(())
}
