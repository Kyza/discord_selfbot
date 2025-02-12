use std::collections::HashMap;

use anyhow::{anyhow, Result};
use byte_unit::rust_decimal::prelude::ToPrimitive;
use heck::ToTitleCase;
use inline_format::format;
use phf::phf_map;
use poise::{
	serenity_prelude::{
		CreateActionRow, CreateAllowedMentions, CreateAttachment,
		CreateButton, CreateEmbed,
	},
	CreateReply,
};
use url::Url;

use crate::config::{Color, Context};

static PLATFORM_CAPITALIZATIONS: phf::Map<&'static str, &'static str> = phf_map! {
	"Youtube" => "YouTube",
	"Youtube Music" => "YouTube Music",
	"Itunes" => "iTunes",
};

#[derive(Debug, Clone)]
pub struct SongPlatformData {
	pub platform_name: String,
	pub song_name: Option<String>,
	pub artist_name: Option<String>,
	pub url: String,
	pub thumbnail_url: Option<String>,
	pub thumbnail_quality: Option<u32>,
}

pub async fn get_song_platform_data(
	ctx: &Context<'_>,
	url: &str,
) -> Result<(String, Vec<SongPlatformData>)> {
	let mut platforms = Vec::new();
	let encoded = urlencoding::encode(url);

	let response = ctx
		.data()
		.http
		.get(format!(
			"https://api.song.link/v1-alpha.1/links?url=",
			encoded
		))
		.send()
		.await?
		.json::<serde_json::Value>()
		.await?;

	let links_by_platform = response["linksByPlatform"].clone();
	let links_by_platform = links_by_platform
		.as_object()
		.ok_or(anyhow!("song.link API returned an unexpected response."))?;
	// entitiesByUniqueId
	let entities_by_unique_id = response["entitiesByUniqueId"].clone();
	let entities_by_unique_id = entities_by_unique_id
		.as_object()
		.ok_or(anyhow!("song.link API returned an unexpected response."))?;

	for (platform, data) in links_by_platform {
		let entity_unique_id = data["entityUniqueId"]
			.as_str()
			.map(|s| s.to_string())
			.ok_or(anyhow!(
				"song.link API returned an unexpected response."
			))?;
		let entity = entities_by_unique_id.get(&entity_unique_id).ok_or(
			anyhow!("song.link API returned an unexpected response."),
		)?;
		let mut platform_name = platform.to_string().to_title_case();
		if let Some(new_name) = PLATFORM_CAPITALIZATIONS.get(&platform_name) {
			platform_name = new_name.to_string();
		}
		platforms.push(SongPlatformData {
			platform_name,
			url: data["url"].as_str().map(|s| s.to_string()).ok_or(
				anyhow!("song.link API returned an unexpected response."),
			)?,
			song_name: entity["title"].as_str().map(|s| s.to_string()),
			artist_name: entity["artistName"].as_str().map(|s| s.to_string()),
			thumbnail_url: entity["thumbnailUrl"]
				.as_str()
				.map(|s| s.to_string()),
			thumbnail_quality: entity["thumbnailWidth"]
				.as_u64()
				.map(|w| {
					(w * entity["thumbnailHeight"].as_u64().unwrap()).to_u32()
				})
				.flatten(),
		});
	}

	Ok((
		response["pageUrl"].as_str().map(|s| s.to_string()).ok_or(
			anyhow!("song.link API returned an unexpected response."),
		)?,
		platforms,
	))
}

pub async fn build_song_info_message(
	ctx: &Context<'_>,
	link: Url,
	song_name: Option<String>,
	artist_name: Option<String>,
	ephemeral: bool,
) -> Result<CreateReply> {
	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	let (page_url, mut platforms) =
		get_song_platform_data(ctx, &link.to_string()).await?;

	let most_common_song_name = song_name
		.or_else(|| {
			platforms
				.iter()
				.filter_map(|platform| platform.song_name.clone())
				.fold(HashMap::new(), |mut map, song| {
					*map.entry(song).or_insert(0) += 1;
					map
				})
				.into_iter()
				.max_by_key(|(_, count)| *count)
				.map(|(song, _)| song.clone())
		})
		.unwrap_or("Unknown".to_string());
	let most_common_artist_name = artist_name
		.or_else(|| {
			platforms
				.iter()
				.filter_map(|platform| platform.artist_name.clone())
				.fold(HashMap::new(), |mut map, song| {
					*map.entry(song).or_insert(0) += 1;
					map
				})
				.into_iter()
				.max_by_key(|(_, count)| *count)
				.map(|(song, _)| song.clone())
		})
		.unwrap_or("Unknown".to_string());

	// Download it and upload it to Discord.
	// Sort by largest thumbnail quality first.
	// Sometimes thumbnails don't download.
	platforms.sort_by(|a, b| {
		b.thumbnail_quality
			.unwrap_or(0)
			.cmp(&a.thumbnail_quality.unwrap_or(0))
	});
	let thumbnail_urls = platforms
		.iter()
		.filter_map(|platform| platform.thumbnail_url.as_ref());
	let mut thumbnail_url_bytes: Option<_> = None;
	for url in thumbnail_urls {
		if let Ok(res) = ctx.data().http.get(url).send().await {
			if let Ok(bytes) = res.bytes().await {
				// Ensure the server actually responded.
				// artwork.anghcdn.co loves ignoring requests.
				if bytes.len() > 0 {
					thumbnail_url_bytes = Some(bytes);
					break;
				}
			}
		}
	}
	if let Some(thumbnail_url_bytes) = thumbnail_url_bytes.clone() {
		reply = reply.attachment(
			CreateAttachment::bytes(thumbnail_url_bytes, "thumbnail.png")
				.description("Cover art."),
		);
	}

	let embed_color = {
		if let Some(thumbnail_url_bytes) = thumbnail_url_bytes {
			use color_thief::{get_palette, ColorFormat};

			let color_bytes = image::load_from_memory(&thumbnail_url_bytes)
				.unwrap()
				.to_rgb8()
				.into_raw();

			get_palette(&color_bytes[..], ColorFormat::Rgb, 10, 2)?
				.iter()
				.next()
				// u8 u8 u8 to u32
				.map(|color| {
					Color(
						color.r as u32
							| (color.g as u32) << 8
							| (color.b as u32) << 16,
					)
				})
				.unwrap_or(ctx.data().config.embed_color.clone())
		} else {
			ctx.data().config.embed_color.clone()
		}
	};
	reply = reply.embed(
		CreateEmbed::new()
			.title(most_common_song_name)
			.url(page_url) // Pick the one with the highest resolution.
			.thumbnail("attachment://thumbnail.png")
			.description(most_common_artist_name)
			.color(embed_color),
	);

	reply = reply.components(
		platforms
			.iter()
			.map(|platform| {
				CreateButton::new_link(platform.url.clone())
					.label(platform.platform_name.clone())
			})
			.collect::<Vec<_>>()
			// 5 is the limit of buttons per row.
			.chunks(5)
			.map(|buttons| CreateActionRow::Buttons(buttons.to_vec()))
			.collect::<Vec<_>>(),
	);

	Ok(reply)
}

/// Shows song information from a given link.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn song_info(
	ctx: Context<'_>,
	#[description = "The link to the song to look up."] url: Url,
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

	ctx.send(
		build_song_info_message(&ctx, url, None, None, ephemeral).await?,
	)
	.await?;
	Ok(())
}
