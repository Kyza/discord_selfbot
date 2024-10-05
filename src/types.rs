use anyhow::Error;
use poise::serenity_prelude as serenity;
use std::env;

#[derive(Debug)]
pub struct Data {
	pub application_id: serenity::UserId,
	pub bot_start_time: std::time::Instant,
	pub http: reqwest::Client,
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
			// godbolt_metadata: std::sync::Mutex::new(
			// 	commands::godbolt::GodboltMetadata::default(),
			// ),
		}
	}
}

pub type Context<'a> = poise::Context<'a, Data, Error>;

// const EMBED_COLOR: (u8, u8, u8) = (0xf7, 0x4c, 0x00);
pub const EMBED_COLOR: (u8, u8, u8) = (0x7b, 0xbe, 0x17);
