use anyhow::Result;
use poise::{
	serenity_prelude::{CreateAllowedMentions, CreateAttachment},
	CreateReply,
};

use crate::types::Context;

/// Queries Wolfram Alpha.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn wolfram(
	ctx: Context<'_>,
	#[description = "Query."] query: String,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let simple_api_url = format!(
		"https://api.wolframalpha.com/v1/simple?i={}&appid={}",
		urlencoding::encode(&query),
		ctx.data().wolfram_alpha_simple_app_id
	);
	let short_api_url = format!(
		"https://api.wolframalpha.com/v1/result?i={}&appid={}",
		urlencoding::encode(&query),
		ctx.data().wolfram_alpha_short_app_id
	);

	let (simple_response, short_response) = (
		ctx.data().http.get(simple_api_url).send().await?,
		ctx.data().http.get(short_api_url).send().await?,
	);

	if !short_response.status().is_success() {
		let body = short_response.text().await?;
		println!("{}", body);
		if body.contains("No short answer available") {
			return Err(anyhow::anyhow!("No short answer available."));
		}
		return Err(anyhow::anyhow!(
			"Failed to get short response from Wolfram Alpha."
		));
	}
	if !simple_response.status().is_success() {
		let body = simple_response.text().await?;
		println!("{}", body);
		return Err(anyhow::anyhow!(
			"Failed to get simple response from Wolfram Alpha."
		));
	}

	let image_bytes = simple_response.bytes().await?;
	let text = short_response.text().await?;

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.attachment(CreateAttachment::bytes(
			image_bytes,
			"wolfram_result.png".to_string(),
		))
		.content(format!("```\n{}\n```", text))
		.ephemeral(ephemeral.unwrap_or(true));

	ctx.send(reply).await?;
	Ok(())
}
