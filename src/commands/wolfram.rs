use std::time::Duration;

use anyhow::{anyhow, Result};
use heck::ToTitleCase;
use indexmap::IndexMap;
use poise::{
	serenity_prelude::{
		CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed,
		CreateInteractionResponse, CreateInteractionResponseFollowup,
		CreateSelectMenuOption,
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
	#[description = "The natural language to query Wolfram Alpha with."]
	query: String,
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

	let query = urlencoding::encode(&query);

	let full_results_api_url = format!(
	   "https://api.wolframalpha.com/v2/query?input={}&format=plaintext,image&output=json&async=false&units=metric&mag=2&appid={}",
		query,
	   ctx.data().wolfram_alpha_full_app_id
	);

	let full_response =
		ctx.data().http.get(full_results_api_url).send().await?;

	if !full_response.status().is_success() {
		return Err(anyhow!(
			"Failed to get response from Wolfram Alpha Full Results API."
		));
	}

	let json: serde_json::Value = full_response.json().await?;

	let pods = json["queryresult"]["pods"]
		.as_array()
		.ok_or_else(|| anyhow!("Invalid response format"))?;

	let mut page_select_options = Vec::new();
	let mut page_map: IndexMap<String, CreateEmbed> = IndexMap::new();

	for pod in pods {
		let title = pod["title"].as_str().unwrap_or("Untitled");
		let subpods = pod["subpods"]
			.as_array()
			.ok_or_else(|| anyhow!("Invalid subpods format"))?;

		for (i, subpod) in subpods.iter().enumerate() {
			let page_title = if subpods.len() > 1 {
				format!("{title} {}", i + 1)
			} else {
				title.to_string()
			}
			.to_title_case();
			let mut page = CreateEmbed::new()
				.color(EMBED_COLOR)
				.title(page_title.clone());

			let plaintext = subpod["plaintext"].as_str();
			if let Some(plaintext) = plaintext {
				if !plaintext.is_empty() {
					let plaintext = format!("```rs\n{}\n```", plaintext);
					page = page.description(plaintext);
				}
			}
			let image_url = subpod["img"]["src"].as_str();
			if let Some(image_url) = image_url {
				page = page.image(image_url);
			}

			page_select_options.push(CreateSelectMenuOption::new(
				page_title.clone(),
				page_title.clone(),
			));

			page_map.insert(page_title.clone(), page.clone());
		}
	}

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.components(vec![CreateActionRow::Buttons(vec![
			CreateButton::new("show_all").label("Show All"),
			CreateButton::new_link(format!(
				"https://www.wolframalpha.com/input?i={}",
				query
			))
			.label("View Online"),
		])])
		.ephemeral(ephemeral);

	page_map.shift_remove("Input");
	if let Some(embed) = page_map.get("Input Interpretation") {
		reply = reply.embed(embed.clone());
		page_map.shift_remove("Input Interpretation");
	}
	if let Some(embed) = page_map.get("Result") {
		reply = reply.embed(embed.clone());
		page_map.shift_remove("Result");
	}

	let message = ctx.send(reply.clone()).await?;

	while match message
		.message()
		.await
		.unwrap()
		.await_component_interaction(&ctx.serenity_context().shard)
		.timeout(Duration::from_secs(60 * 5))
		.await
	{
		Some(ref interaction) => {
			if interaction.data.custom_id == "show_all" {
				interaction
					.create_response(
						ctx,
						CreateInteractionResponse::Acknowledge,
					)
					.await?;
				let next = page_map.clone();
				let next = next.values();
				let next = next.collect::<Vec<&CreateEmbed>>();
				let chunks = next.chunks(10);
				for chunk in chunks {
					let mut followup =
						CreateInteractionResponseFollowup::new();
					for embed in chunk {
						followup = followup.add_embed((*embed).clone());
					}
					interaction
						.create_followup(ctx, followup.ephemeral(true))
						.await?;
				}
			}
			true
		}
		None => false,
	} {}

	Ok(())
}
