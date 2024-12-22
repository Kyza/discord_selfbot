#[macro_use]
extern crate maplit;

use std::env;

use poise::serenity_prelude as serenity;
use types::Data;

pub mod commands;
pub mod helpers;
pub mod key_value_args_utils;
pub mod media;
pub mod os_command;
pub mod types;

#[dotenvy::load]
#[tokio::main]
async fn main() {
	let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN is required");
	let intents = serenity::GatewayIntents::non_privileged();

	let framework = poise::Framework::builder()
		.options(poise::FrameworkOptions {
			owners: hashset! { serenity::UserId::new(220584715265114113) },
			commands: vec![
				commands::age(),
				commands::github(),
				commands::fix(),
				commands::crate_(),
				commands::doc(),
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
				commands::rust_playground(),
				commands::favoritize(),
				commands::favoritize_context_menu(),
				commands::webp(),
				commands::webp_context_menu(),
				// commands::microbench(),
				// commands::miri(),
				// commands::expand(),
				// commands::clippy(),
				// commands::fmt(),
				// commands::rust_playground_context_menu(),
				// commands::playwarn(),
				// commands::playwarn_context_menu(),
				// commands::eval(),
				// commands::eval_context_menu(),
				// commands::procmacro(),
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
				Ok(Data::new())
			})
		})
		.build();

	let client = serenity::ClientBuilder::new(token, intents)
		.framework(framework)
		.await;
	client.unwrap().start().await.unwrap();
}
