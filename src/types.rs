use anyhow::Error;
use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs};

#[derive(Debug)]
pub struct BotData {
	pub config: Config,
	pub http: reqwest::Client,
	pub bot_start_time: std::time::Instant,
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
	pub wolfram_alpha_full_app_id: String,
	// pub wolfram_alpha_simple_app_id: String,
	// pub wolfram_alpha_short_app_id: String,
	pub deepl_target_language: String,
	pub timezone: String,
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
