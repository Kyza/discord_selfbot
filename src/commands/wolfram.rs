use std::time::Duration;

use anyhow::{anyhow, Result};
use heck::{ToSnakeCase, ToTitleCase};
use indexmap::IndexMap;
use inline_format::format;
use poise::{
	serenity_prelude::{
		CreateActionRow, CreateAllowedMentions, CreateAttachment,
		CreateButton, CreateEmbed, CreateInteractionResponse,
		CreateInteractionResponseFollowup, Mentionable,
	},
	CreateReply,
};

const EMBED_COLOR: u32 = 0xff6600;

use crate::config::Context;

pub fn generate_timeouts(time: Duration) -> String {
	format!(
		"&scantimeout=",
		time.as_secs(),
		"&podtimeout=",
		time.as_secs(),
		"&formattimeout=",
		time.as_secs(),
		"&parsetimeout=",
		time.as_secs(),
		"&totaltimeout=",
		time.as_secs() * 4,
	)
}

#[derive(Debug)]
pub enum Pod {
	Plaintext(String),
	Image(String),
}

/// Asks Wolfram Alpha a question.
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

	let wolfram_alpha_full_app_id = if let Some(app_id) =
		ctx.data().config.wolfram_alpha_full_app_id.clone()
	{
		app_id
	} else {
		return Err(anyhow!("wolfram_alpha_full_app_id is not set."));
	};

	let query = urlencoding::encode(&query);

	let full_results_api_url = format!(
	   "https://api.wolframalpha.com/v2/query?input=",
		query,
		generate_timeouts(Duration::from_secs(60)),
		"&format=plaintext,image&output=json&async=false&units=metric&mag=2&appid=",
	   wolfram_alpha_full_app_id
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

	let mut page_map: IndexMap<
		String,
		(CreateEmbed, Option<CreateAttachment>),
	> = IndexMap::new();

	for pod in pods {
		let title = pod["title"].as_str().unwrap_or("Untitled");
		let subpods = pod["subpods"]
			.as_array()
			.ok_or_else(|| anyhow!("Invalid subpods format"))?;

		for (i, subpod) in subpods.iter().enumerate() {
			let page_title = if subpods.len() > 1 {
				format!(title, " ", i + 1)
			} else {
				title.to_string()
			}
			.to_title_case();
			let mut page = CreateEmbed::new()
				.color(EMBED_COLOR)
				.title(page_title.clone());

			let mut had_plaintext = false;
			let mut had_image = false;
			let mut image = None;

			let plaintext = subpod["plaintext"].as_str();
			if let Some(plaintext) = plaintext {
				if !plaintext.is_empty() {
					had_plaintext = true;
					let plaintext = format!("```rs\n", plaintext, "\n```");
					page = page.description(plaintext);
				}
			}
			let image_url = subpod["img"]["src"].as_str();
			if let Some(image_url) = image_url {
				had_image = true;
				// Download the image so that it doesn't expire and supports
				// alt text for whenever Discord remembers to add it to embeds.
				let image_name = format!(page_title.to_snake_case(), ".webp");
				let mut image_attachment = CreateAttachment::bytes(
					ctx.data()
						.http
						.get(image_url)
						.send()
						.await?
						.bytes()
						.await?,
					image_name.clone(),
				);
				if had_plaintext {
					if let Some(plaintext) = plaintext {
						image_attachment =
							image_attachment.description(plaintext);
					}
				}
				image = Some(image_attachment);
				page = page.attachment(image_name);
			}

			if !had_plaintext && !had_image {
				page =
					page.description("Pod didn't have any parsable content.\nView it online to see the full result.");
			}

			page_map.insert(page_title.clone(), (page.clone(), image));
		}
	}

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.components(vec![CreateActionRow::Buttons(vec![
			CreateButton::new("show_all_private").label("Show All (Private)"),
			CreateButton::new("show_all_public").label("Show All (Public)"),
			CreateButton::new_link(format!(
				"https://www.wolframalpha.com/input?i=",
				query
			))
			.label("View Online"),
		])])
		.ephemeral(ephemeral);

	page_map.shift_remove("Input");
	let mut added_embed = false;
	if let Some((embed, attachment)) = page_map.get("Input Interpretation") {
		reply = reply.embed(embed.clone());
		if let Some(attachment) = attachment {
			reply = reply.attachment(attachment.clone());
		}
		page_map.shift_remove("Input Interpretation");
		added_embed = true;
	}
	if let Some((embed, attachment)) = page_map.get("Result") {
		reply = reply.embed(embed.clone());
		if let Some(attachment) = attachment {
			reply = reply.attachment(attachment.clone());
		}
		page_map.shift_remove("Result");
		added_embed = true;
	}
	// As a backup, add the first embed.
	if !added_embed {
		// If the default embed wasn't added, add the first one.
		if let Some((embed, attachment)) = page_map.values().next() {
			reply = reply.embed(embed.clone());
			if let Some(attachment) = attachment {
				reply = reply.attachment(attachment.clone());
			}
			// Remove the first embed from the map.
			if let Some(first_key) = page_map.keys().next().cloned() {
				page_map.shift_remove(&first_key);
			}
		}
	}

	let message = ctx.send(reply.clone()).await?;

	let mut responded_publicly = false;

	'interaction_loop: while match message
		.message()
		.await
		.unwrap()
		.await_component_interaction(&ctx.serenity_context().shard)
		.timeout(Duration::from_secs(60 * 5))
		.await
	{
		Some(ref interaction) => {
			let show_all_private =
				interaction.data.custom_id == "show_all_private";
			let show_all_public =
				interaction.data.custom_id == "show_all_public";

			// If already responded to the public, don't respond again.
			if show_all_public && responded_publicly {
				interaction
					.create_response(
						ctx,
						CreateInteractionResponse::Acknowledge,
					)
					.await?;

				let mut followup = CreateInteractionResponseFollowup::new();
				followup =
					followup.content("There has already been a public response.\nIf it was deleted, use the private response or view the results online.");
				interaction
					.create_followup(ctx, followup.ephemeral(true))
					.await?;

				continue 'interaction_loop;
			}
			if show_all_public {
				responded_publicly = true;
			}

			if show_all_private || show_all_public {
				interaction
					.create_response(
						ctx,
						CreateInteractionResponse::Acknowledge,
					)
					.await?;

				let next = page_map.clone();
				let next = next.values();
				let next = next
					.collect::<Vec<&(CreateEmbed, Option<CreateAttachment>)>>(
					);
				let chunks = next.chunks(10);
				for chunk in chunks {
					let mut followup =
						CreateInteractionResponseFollowup::new();
					if show_all_public {
						followup = followup
							.content(interaction.user.mention().to_string());
					}
					for (embed, attachment) in chunk {
						followup = followup.add_embed((*embed).clone());
						if let Some(attachment) = attachment {
							followup = followup.add_file(attachment.clone());
						}
					}
					interaction
						.create_followup(
							ctx,
							followup.ephemeral(show_all_private),
						)
						.await?;
				}
			}
			true
		}
		None => false,
	} {}

	Ok(())
}
