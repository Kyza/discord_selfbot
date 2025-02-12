use anyhow::Result;
use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

use crate::config::Context;

/// Flips a nickel using a true random number generator.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn flip(
	ctx: Context<'_>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let ephemeral = ephemeral.unwrap_or(false);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

	let api_key =
		if let Some(api_key) = ctx.data().config.randomorg_api_key.clone() {
			api_key
		} else {
			return Err(anyhow::anyhow!("RANDOM.ORG API key not set."));
		};

	let response = ctx
		.data()
		.http
		.post("https://api.random.org/json-rpc/4/invoke")
		.json(&serde_json::json!({
			"jsonrpc": "2.0",
			"method": "generateIntegers",
			"params": {
				"apiKey": api_key,
				"n": 1,
				"min": 1,
				"max": 12000,
				"replacement": true,
				"base": 10
			},
			"id": 1
		}))
		.send()
		.await?
		.json::<serde_json::Value>()
		.await?;

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	let api_result = response["result"]["random"]["data"][0]
		.clone()
		.as_i64()
		.unwrap_or(12001);

	// A 1/6,000 chance of it landing on its side.
	match api_result {
		1..=5999 => {
			reply = reply.content("Heads.");
		}
		6000..=11998 => {
			reply = reply.content("Tails.");
		}
		11999..=12000 => {
			reply = reply.content("Edge.");
		}
		_ => {
			reply = reply.content(
				"The nickel went up and never landed. It's gone now. I can't find it.",
			);
		}
	}

	ctx.send(reply).await?;
	Ok(())
}
