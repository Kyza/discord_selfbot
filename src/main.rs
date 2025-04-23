#[global_allocator]
static ALLOC: MiMalloc = MiMalloc;

use std::sync::Arc;

use config::{BotData, Config};
use inline_format::eprintln;
use mimalloc::MiMalloc;
use poise::serenity_prelude as serenity;

pub mod commands;
// pub mod component_count;
pub mod config;
pub mod helpers;
pub mod media;
pub mod os_command;
pub mod youtube_downloader;

#[tokio::main]
async fn main() {
	let config = Config::new();
	let intents = serenity::GatewayIntents::non_privileged();

	let mut commands = vec![
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
		commands::translate(),
		// commands::embed(),
		commands::screenshot(),
		commands::flip(),
		commands::now_playing(),
		commands::song_info(),
		commands::source(),
	];

	// Add the context menu commands if they're in the config.
	for command_name in &config.context_menu_commands {
		match command_name.as_str() {
			"bible" => {
				commands.push(commands::bible_context_menu());
			}
			"song_info" => {
				commands.push(commands::song_info_context_menu());
			}
			"favoritize" => {
				commands.push(commands::favoritize_context_menu());
			}
			"translate" => {
				// commands.push(commands::translate_context_menu());
			}
			"webp" => {
				commands.push(commands::webp_context_menu());
			}
			"jxl" => {
				commands.push(commands::jxl_context_menu());
			}
			name => {
				eprintln!("Warning! Command \"", name, "\" doesn't exist.");
			}
		}
	}

	let options = poise::FrameworkOptions {
		owners: config.owner_ids.clone(),
		commands,
		..Default::default()
	};

	let framework = poise::Framework::builder()
		.options(options)
		// .setup(|ctx, _ready, framework| {
		// 	Box::pin(async move {
		// 		poise::builtins::register_globally(
		// 			ctx,
		// 			&framework.options().commands,
		// 		)
		// 		.await?;
		// 		Ok(BotData::new())
		// 	})
		// })
		.build();

	let client =
		serenity::ClientBuilder::new(config.discord_token.clone(), intents)
			.framework(framework)
			.data(Arc::new(BotData::new()))
			.await;
	client.unwrap().start().await.unwrap();
}
