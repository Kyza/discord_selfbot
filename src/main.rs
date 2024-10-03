#[macro_use]
extern crate maplit;

use std::env;

use poise::serenity_prelude as serenity;
use types::Data;

pub mod commands;
pub mod helpers;
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
				commands::cobalt(),
				commands::crate_(),
				commands::doc(),
				commands::cleanup(),
				commands::uptime(),
				commands::help(),
				commands::timestamp(),
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
