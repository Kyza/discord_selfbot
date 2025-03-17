use anyhow::Error;
use chrono::{DateTime, Local};
use chrono_tz::Tz;
use poise::serenity_prelude::{self as serenity, Colour};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs};

#[derive(Debug, Clone)]
pub struct TimeZoneMiddleman(Tz);
impl TimeZoneMiddleman {
	pub fn local(&self) -> DateTime<Tz> {
		Local::now().with_timezone(&self.0)
	}
}
impl Serialize for TimeZoneMiddleman {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(self.0.name())
	}
}
impl<'de> Deserialize<'de> for TimeZoneMiddleman {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		s.parse::<Tz>()
			.map(TimeZoneMiddleman)
			.map_err(serde::de::Error::custom)
	}
}

/// No more British.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color(pub u32);
impl From<Color> for Colour {
	fn from(color: Color) -> Self {
		Colour::from(color.0)
	}
}

#[derive(Debug)]
pub struct BotData {
	pub config: Config,
	pub http: reqwest::Client,
	pub bot_start_time: std::time::Instant,
}
impl Default for BotData {
	fn default() -> Self {
		Self::new()
	}
}

impl BotData {
	pub fn new() -> Self {
		let config = Config::new();
		let http = reqwest::Client::new();
		let bot_start_time = std::time::Instant::now();
		Self {
			config,
			http,
			bot_start_time,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	pub discord_token: String,
	pub owner_ids: HashSet<serenity::UserId>,
	pub application_id: serenity::UserId,
	pub wolfram_alpha_full_app_id: Option<String>,
	// pub wolfram_alpha_simple_app_id: Option<String>,
	// pub wolfram_alpha_short_app_id: Option<String>,
	pub deepl_target_language: String,
	pub timezone: TimeZoneMiddleman,
	pub embed_color: Color,
	pub randomorg_api_key: Option<String>,
	pub listenbrainz_user: Option<String>,
	pub context_menu_commands: Vec<String>,
}
impl Default for Config {
	fn default() -> Self {
		Self::new()
	}
}
impl Config {
	pub fn new() -> Self {
		let config_string = fs::read_to_string("config.ron").unwrap();
		ron::from_str(&config_string).unwrap()
	}
}

pub type Context<'a> = poise::Context<'a, BotData, Error>;
pub type ApplicationContext<'a> =
	poise::ApplicationContext<'a, BotData, Error>;
