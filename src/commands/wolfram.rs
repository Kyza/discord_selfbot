use anyhow::{anyhow, Result};
use poise::{
	serenity_prelude::{
		CreateAllowedMentions, CreateAttachment, CreateEmbed,
	},
	CreateReply,
};

const EMBED_COLOR: u32 = 0xff6600;

use crate::types::Context;

#[derive(Debug)]
pub enum Pod {
	Plaintext(String),
	Image(String),
}

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
	#[description = "Whether or not to send images."] images: Option<bool>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	ctx.defer().await?;

	let full_results_api_url = format!(
      "https://api.wolframalpha.com/v2/query?input={}&format=plaintext,image&reinterpret=true&output=JSON&appid={}",
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

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral.unwrap_or(false));

	let mut main_embed = CreateEmbed::default().color(EMBED_COLOR);

	let mut embeds = Vec::new();

	let mut attachment_count = 0;

	for pod in pods {
		let title = pod["title"].as_str().unwrap_or("Untitled");
		let subpods = pod["subpods"]
			.as_array()
			.ok_or_else(|| anyhow!("Invalid subpods format"))?;

		for (index, subpod) in subpods.iter().enumerate() {
			let plaintext = subpod["plaintext"].as_str();
			let image = if let Some(image_url) = subpod["img"]["src"].as_str()
			{
				Some(image_url)
			} else {
				None
			};
			let image_type = subpod["img"]["type"].as_str();

			if !images.unwrap_or(true) {
				if let Some(plaintext) = plaintext {
					main_embed = main_embed.field(
						if index == 0 { title } else { "" },
						&format!("```rs\n{}\n```", plaintext),
						false,
					);
				}
			} else {
				match (plaintext, image, image_type) {
					(Some(plaintext), _, Some("Default") | None) => {
						if !plaintext.is_empty() {
							main_embed = main_embed.field(
								if index == 0 { title } else { "" },
								&format!("```rs\n{}\n```", plaintext),
								false,
							);
							// has_plaintext = true;
							// pod_content
							// 	.push_str(&format!("```rs\n{}\n```", plaintext));
						}
					}
					(_, Some(image), _) => {
						// Sometimes the image doesn't load, so download it because maybe that'll fix it.
						let image_bytes = ctx
							.data()
							.http
							.get(image)
							.send()
							.await?
							.bytes()
							.await?;
						let image_name = format!(
							"wolfram_result_{}.gif",
							attachment_count
						);
						attachment_count += 1;
						let image = CreateAttachment::bytes(
							image_bytes,
							image_name.clone(),
						);

						reply = reply.attachment(image);

						let embed = CreateEmbed::default()
							.title(title)
							.attachment(image_name)
							.color(EMBED_COLOR);
						embeds.push(embed);
					}
					(Some(plaintext), _, _) => {
						if !plaintext.is_empty() {
							main_embed = main_embed.field(
								if index == 0 { title } else { "" },
								&format!("```rs\n{}\n```", plaintext),
								false,
							);
							// has_plaintext = true;
							// pod_content
							// 	.push_str(&format!("```rs\n{}\n```", plaintext));
						}
					}
					_ => (),
				};
			}
		}
	}

	println!("Embeds: {}", embeds.len());

	reply = reply.embed(main_embed);

	for (index, embed) in embeds.iter().enumerate() {
		if index == 9 {
			break;
		}
		reply = reply.embed(embed.clone());
	}

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
