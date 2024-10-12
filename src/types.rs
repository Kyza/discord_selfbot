use anyhow::Error;
use poise::serenity_prelude as serenity;
use std::env;

#[derive(Debug)]
pub struct Data {
	pub application_id: serenity::UserId,
	pub bot_start_time: std::time::Instant,
	pub http: reqwest::Client,
	pub wolfram_alpha_full_app_id: String,
	pub wolfram_alpha_simple_app_id: String,
	// pub tz: Tz,
	// pub godbolt_metadata:
	// 	std::sync::Mutex<commands::godbolt::GodboltMetadata>,
}

impl Data {
	pub fn new() -> Self {
		Self {
			application_id: env::var("APPLICATION_ID")
				.expect("APPLICATION_ID is required")
				.parse()
				.expect("APPLICATION_ID is not a valid ID"),
			bot_start_time: std::time::Instant::now(),
			http: reqwest::Client::new(),
			wolfram_alpha_full_app_id: env::var("WOLFRAM_ALPHA_FULL_APP_ID")
				.expect("WOLFRAM_ALPHA_FULL_APP_ID is required"),
			wolfram_alpha_simple_app_id: env::var(
				"WOLFRAM_ALPHA_SIMPLE_APP_ID",
			)
			.expect("WOLFRAM_ALPHA_SIMPLE_APP_ID is required"),
			// tz: env::var("TIMEZONE")
			// 	.expect("TIMEZONE is required")
			// 	.parse()
			// 	.expect("TIMEZONE is invalid"),
			// godbolt_metadata: std::sync::Mutex::new(
			// 	commands::godbolt::GodboltMetadata::default(),
			// ),
		}
	}
}

pub type Context<'a> = poise::Context<'a, Data, Error>;

// const EMBED_COLOR: (u8, u8, u8) = (0xf7, 0x4c, 0x00);
pub const EMBED_COLOR: (u8, u8, u8) = (0x7b, 0xbe, 0x17);
