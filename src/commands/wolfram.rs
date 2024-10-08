use anyhow::{anyhow, Result};
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
	let full_results_api_url = format!(
       "https://api.wolframalpha.com/v2/query?input={}&format=plaintext,image&output=JSON&appid={}",
       urlencoding::encode(&query),
       ctx.data().wolfram_alpha_full_app_id
   );

	let response = ctx.data().http.get(full_results_api_url).send().await?;

	if !response.status().is_success() {
		return Err(anyhow!(
			"Failed to get response from Wolfram Alpha Full Results API."
		));
	}

	let json: serde_json::Value = response.json().await?;
	let pods = json["queryresult"]["pods"]
		.as_array()
		.ok_or_else(|| anyhow!("Invalid response format"))?;

	let mut markdown = String::new();

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral.unwrap_or(false));

	for pod in pods {
		let mut images = Vec::new();

		let title = pod["title"].as_str().unwrap_or("Untitled");
		let subpods = pod["subpods"]
			.as_array()
			.ok_or_else(|| anyhow!("Invalid subpods format"))?;

		let mut pod_content = String::new();
		let mut has_plaintext = false;

		for (_index, subpod) in subpods.iter().enumerate() {
			if let Some(plaintext) = subpod["plaintext"].as_str() {
				if !plaintext.is_empty() {
					has_plaintext = true;
					pod_content.push_str(&format!("```\n{}\n```", plaintext));
				}
			}

			if let Some(image_url) = subpod["img"]["src"].as_str() {
				let image_response =
					ctx.data().http.get(image_url).send().await?;
				if image_response.status().is_success() {
					let image_bytes = image_response.bytes().await?;
					let image_filename =
						format!("wolfram_result_{}.gif", images.len() + 1);
					images.push(CreateAttachment::bytes(
						image_bytes,
						image_filename.clone(),
					));
				}
			}
		}

		if has_plaintext {
			markdown.push_str(&format!("**__{}:__**\n", title));
			markdown.push_str(&pod_content);
		} else if !pod_content.is_empty() {
			markdown.push_str(&pod_content);
		} else {
			for image in images {
				reply = reply.attachment(image);
			}
		}
	}

	reply = reply.content(markdown);

	ctx.send(reply).await?;
	Ok(())

	// let simple_api_url = format!(
	// 	"https://api.wolframalpha.com/v1/simple?i={}&appid={}",
	// 	urlencoding::encode(&query),
	// 	ctx.data().wolfram_alpha_simple_app_id
	// );
	// let short_api_url = format!(
	// 	"https://api.wolframalpha.com/v1/result?i={}&appid={}",
	// 	urlencoding::encode(&query),
	// 	ctx.data().wolfram_alpha_short_app_id
	// );

	// let (simple_response, short_response) = (
	// 	ctx.data().http.get(simple_api_url).send().await?,
	// 	ctx.data().http.get(short_api_url).send().await?,
	// );

	// if !short_response.status().is_success() {
	// 	let body = short_response.text().await?;
	// 	println!("{}", body);
	// 	if body.contains("No short answer available") {
	// 		return Err(anyhow::anyhow!("No short answer available."));
	// 	}
	// 	return Err(anyhow::anyhow!(
	// 		"Failed to get short response from Wolfram Alpha."
	// 	));
	// }
	// if !simple_response.status().is_success() {
	// 	let body = simple_response.text().await?;
	// 	println!("{}", body);
	// 	return Err(anyhow::anyhow!(
	// 		"Failed to get simple response from Wolfram Alpha."
	// 	));
	// }

	// let image_bytes = simple_response.bytes().await?;
	// let text = short_response.text().await?;

	// let reply = CreateReply::default()
	// 	.allowed_mentions(CreateAllowedMentions::default())
	// 	.attachment(CreateAttachment::bytes(
	// 		image_bytes,
	// 		"wolfram_result.png".to_string(),
	// 	))
	// 	.content(format!("```\n{}\n```", text))
	// 	.ephemeral(ephemeral.unwrap_or(false));

	// ctx.send(reply).await?;
	// Ok(())
}
