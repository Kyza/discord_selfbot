use poise::serenity_prelude as serenity;
use types::{BotData, Config};

pub mod commands;
pub mod helpers;
pub mod key_value_args_utils;
pub mod media;
pub mod os_command;
pub mod types;

#[tokio::main]
async fn main() {
	let config = Config::new();
	let intents = serenity::GatewayIntents::non_privileged();

	let framework = poise::Framework::builder()
		.options(poise::FrameworkOptions {
			owners: config.owner_ids.clone(),
			commands: vec![
				commands::age(),
				commands::github(),
				commands::fix(),
				commands::uptime(),
				commands::help(),
				commands::snowstamp(),
				commands::wolfram(),
				commands::wayback(),
				commands::unicode(),
				commands::escape(),
				commands::roll(),
				commands::youtube(),
				commands::ocr(),
				commands::bible(),
				commands::favoritize(),
				commands::favoritize_context_menu(),
				commands::webp(),
				commands::webp_context_menu(),
				commands::ffmpeg(),
				commands::translate(),
				commands::translate_context_menu(),
			],
			..Default::default()
		})
		.setup(|ctx, _ready, framework| {
			Box::pin(async move {
				poise::builtins::register_globally(
					ctx,
					&framework.options().commands,
				)
				.await?;
				Ok(BotData::new())
			})
		})
		.build();

	let client =
		serenity::ClientBuilder::new(config.discord_token.clone(), intents)
			.framework(framework)
			.await;
	client.unwrap().start().await.unwrap();
}
