#[global_allocator]
static ALLOC: MiMalloc = MiMalloc;

use std::{str::FromStr, sync::Arc};

use anyhow::Result;
use config::{BotData, Config, Error};
use inline_format::{eprintln, println};
use mimalloc::MiMalloc;
use poise::{
	samples::create_application_commands,
	serenity_prelude::{
		self as serenity, ActivityData, ClientBuilder, OnlineStatus, Token,
	},
	Command, FrameworkOptions,
};
use secrecy::ExposeSecret;

pub mod commands;
// pub mod component_count;
pub mod config;
pub mod helpers;
pub mod media;
pub mod os_command;
pub mod youtube_downloader;

pub fn get_commands() -> Vec<Command<BotData, Error>> {
	vec![
		commands::age(),
		commands::github(),
		commands::fix(),
		commands::uptime(),
		// commands::help(),
		commands::snowstamp(),
		commands::wolfram(),
		commands::wayback(),
		commands::unicode(),
		commands::escape(),
		commands::roll(),
		commands::youtube(),
		commands::ocr(),
		commands::favoritize(),
		commands::webp(),
		commands::jxl(),
		commands::ffmpeg(),
		commands::deepl(),
		// commands::deepl_translate(),
		// commands::deepl_usage(),
		// commands::embed(),
		commands::screenshot(),
		commands::flip(),
		commands::now_playing(),
		commands::song_info(),
		commands::source(),
		commands::command_buttons(),
	]
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let config = Config::new();
	println!(config:#?);
	let intents = serenity::GatewayIntents::non_privileged();

	let commands = get_commands();

	// Add the context menu commands if they're in the config.
	// for command_name in &config.context_menu_commands {
	// 	match command_name.as_str() {
	// 		"bible" => {
	// 			commands.push(commands::bible_context_menu());
	// 		}
	// 		"song_info" => {
	// 			commands.push(commands::song_info_context_menu());
	// 		}
	// 		"favoritize" => {
	// 			commands.push(commands::favoritize_context_menu());
	// 		}
	// 		"translate" => {
	// 			// commands.push(commands::translate_context_menu());
	// 		}
	// 		"webp" => {
	// 			commands.push(commands::webp_context_menu());
	// 		}
	// 		"jxl" => {
	// 			commands.push(commands::jxl_context_menu());
	// 		}
	// 		name => {
	// 			eprintln!("Warning! Command \"", name, "\" doesn't exist.");
	// 		}
	// 	}
	// }

	let options: FrameworkOptions<BotData, Error> = poise::FrameworkOptions {
		owners: config.owner_ids.clone(),
		commands,
		..Default::default()
	};

	let framework = poise::Framework::builder().options(options).build();

	let mut client = ClientBuilder::new(
		Token::from_str(config.discord_token.expose_secret())
			.expect("Invalid Discord token"),
		intents,
	)
	.activity(ActivityData::competing("Discord against Kyza."))
	.status(OnlineStatus::DoNotDisturb)
	.framework(framework)
	.data(Arc::new(BotData::new()))
	.await?;

	client.http.set_application_id(config.application_id);

	let commands =
		create_application_commands::<BotData, Error>(&get_commands());
	if let Err(err) = client.http.create_global_commands(&commands).await {
		eprintln!("Error creating global commands: ", err);
	}

	client.start_autosharded().await.map_err(|e| e.into())
}
