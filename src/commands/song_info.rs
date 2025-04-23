use std::{collections::HashMap, sync::LazyLock, time::Duration};

use anyhow::{anyhow, Result};
use byte_unit::rust_decimal::prelude::ToPrimitive;
use heck::ToTitleCase;
use inline_format::format;
use phf::phf_map;
use poise::{
	serenity_prelude::{
		futures::{
			future::{self, Either},
			FutureExt,
		},
		ComponentInteractionCollector, CreateActionRow,
		CreateAllowedMentions, CreateAttachment, CreateButton,
		CreateComponent, CreateContainer, CreateInteractionResponse,
		CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
		CreateSection, CreateSectionAccessory, CreateSectionComponent,
		CreateTextDisplay, CreateThumbnail, CreateUnfurledMediaItem, Message,
		MessageFlags,
	},
	CreateReply, Modal, ReplyHandle,
};
use regex::Regex;
use thirtyfour::{
	prelude::{ElementQueryable, ElementWaitable},
	By, DesiredCapabilities, WebDriver,
};
use url::Url;

use crate::{
	config::{ApplicationContext, Color, Context},
	helpers::{escape_markdown, wait_for_element, ContentOrAttachmentExt},
	youtube_downloader::{DownloadFormat, YouTubeDownloader},
};

static PLATFORM_CAPITALIZATIONS: phf::Map<&'static str, &'static str> = phf_map! {
	"Youtube" => "YouTube",
	"Youtube Music" => "YouTube Music",
	"Itunes" => "iTunes",
	"Soundcloud" => "SoundCloud",
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

	if let Some(status_code) = response["statusCode"].as_u64() {
		if status_code != 200 {
			let error_code =
				response["code"].as_str().unwrap_or("unknown_code");
			return Err(anyhow!(
				"song.link API returned an error: `{}`",
				error_code
			));
		}
	}

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
			thumbnail_quality: entity["thumbnailWidth"].as_u64().and_then(
				|w| {
					(w * entity["thumbnailHeight"].as_u64().unwrap()).to_u32()
				},
			),
		});
	}

	Ok((
		response["pageUrl"].as_str().map(|s| s.to_string()).ok_or(
			anyhow!("song.link API returned an unexpected response."),
		)?,
		platforms,
	))
}

pub async fn build_song_info_message<'a>(
	ctx: &Context<'a>,
	link: Url,
	song_name: Option<String>,
	artist_name: Option<String>,
	youtube_downloader: &mut YouTubeDownloader,
	ephemeral: bool,
) -> Result<(CreateReply<'a>, CreateReply<'a>)> {
	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.flags(MessageFlags::IS_COMPONENTS_V2)
		.ephemeral(ephemeral);

	let (page_url, mut platforms) =
		get_song_platform_data(ctx, link.as_ref()).await?;

	// Search for a YouTube link...
	// Priority: YouTube Music > YouTube
	let youtube_link = 'ytl: {
		if let Some(url) = youtube_downloader.get_url() {
			break 'ytl Some(url);
		}
		let mut youtube_url = None;
		for platform in platforms.iter_mut() {
			match platform.platform_name.as_str() {
				"YouTube" => {
					youtube_url =
						Some(Url::parse(platform.url.as_str()).unwrap());
				}
				"YouTube Music" => {
					break 'ytl Some(
						Url::parse(platform.url.as_str()).unwrap(),
					);
				}
				_ => {}
			};
		}
		youtube_url
	};
	if let Some(youtube_link) = youtube_link {
		// Start the song download.
		if !youtube_downloader.is_downloading() {
			youtube_downloader.url = Some(youtube_link);
		}
		youtube_downloader.format = DownloadFormat::Audio;
		let _ = youtube_downloader.start_download();
	} else {
		println!("No YouTube link found. Can't download song.");
	}

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

	youtube_downloader.file_name(format!(
		most_common_song_name,
		" by ", most_common_artist_name, ".mp3"
	));

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
				if !bytes.is_empty() {
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

	let accent_color = {
		if let Some(thumbnail_url_bytes) = thumbnail_url_bytes {
			use color_thief::{get_palette, ColorFormat};

			let color_bytes = image::load_from_memory(&thumbnail_url_bytes)
				.unwrap()
				.to_rgb8()
				.into_raw();

			get_palette(&color_bytes[..], ColorFormat::Rgb, 10, 2)?
				.first()
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
	let make_components = |disabled| {
		let mut components =
			vec![CreateComponent::Section(CreateSection::new(
				[CreateSectionComponent::TextDisplay(
					CreateTextDisplay::new(format!(
						"## [",
						escape_markdown(&most_common_song_name),
						"](",
						page_url,
						")\n### by ",
						escape_markdown(&most_common_artist_name),
					)),
				)]
				.to_vec()
				.to_owned(),
				CreateSectionAccessory::Thumbnail(CreateThumbnail::new(
					CreateUnfurledMediaItem::new(
						"attachment://thumbnail.png",
					),
				)),
			))];
		components.append(
			&mut platforms
				.iter()
				.map(|platform| {
					CreateButton::new_link(platform.url.clone())
						.label(platform.platform_name.clone())
				})
				.collect::<Vec<_>>()
				// 5 is the limit of buttons per row.
				.chunks(5)
				.map(|buttons| {
					CreateComponent::ActionRow(CreateActionRow::Buttons(
						buttons.to_vec().into(),
					))
				})
				.collect::<Vec<_>>(),
		);
		if youtube_downloader.was_started() {
			components.push(CreateComponent::ActionRow(
				CreateActionRow::Buttons(
					[CreateButton::new("audio")
						.label("Send Audio")
						.disabled(disabled)]
					.to_vec()
					.into(),
				),
			));
		}
		components
	};
	reply = reply.components(
		[CreateComponent::Container(
			CreateContainer::new(make_components(false))
				.accent_color(accent_color.clone()),
		)]
		.to_vec()
		.to_owned(),
	);

	let disabled_reply = reply.clone().components(
		[CreateComponent::Container(
			CreateContainer::new(make_components(true))
				.accent_color(accent_color),
		)]
		.to_vec()
		.to_owned(),
	);

	Ok((reply, disabled_reply))
}

/// Uses Firefox to search https://odesli.co/.
pub async fn get_song_link_searchable_link(query: String) -> Result<Url> {
	// Search input:
	// #search-page-downshift-input
	// Result:
	// #search-page-downshift-item-0 a

	println!("Starting Firefox.");

	let mut caps = DesiredCapabilities::firefox();
	// If in debug mode, run non-headless.
	// Look at him he has Smitty Werbenjägermanjensen's hat.
	if !cfg!(debug_assertions) {
		caps.set_headless()?;
	}
	// caps.set_log_level(LogLevel::Trace)?;
	let driver = WebDriver::new("http://localhost:4444", caps).await?;

	let starting_url = format!("https://odesli.co/");
	driver.goto(starting_url.clone()).await?;

	println!("Firefox started.");

	// The page is done loading when the search input is clickable.
	let search_input =
		wait_for_element(&driver, "#search-page-downshift-input").await?;
	search_input.wait_until().clickable().await?;
	// Type the song name, space, artist name.
	search_input.send_keys(&query).await?;

	// Wait for the search results to load.
	// OR if the page contains the text "No results found".
	let first_search_result =
		wait_for_element(&driver, "#search-page-downshift-item-0 a");
	let no_results_text = driver
		.query(By::XPath("//div[contains(text(), 'No results found')]"))
		.wait(Duration::from_secs(60), Duration::from_millis(10));
	let no_results_text = no_results_text.first();

	let result =
		future::select(first_search_result.boxed(), no_results_text.boxed())
			.await;

	match result {
		Either::Left((Ok(first_search_result), _)) => {
			// Get the URL.
			let url = first_search_result.attr("href").await?;

			if let Some(url) = url {
				Ok(Url::parse(&url)?)
			} else {
				Err(anyhow!("No search results found."))
			}
		}
		Either::Left((_, _)) => {
			Err(anyhow!("Fatal error finding search results."))
		}
		Either::Right((_, _)) => Err(anyhow!("No search results found.")),
	}
}

static LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
	Regex::new(r#"(https?:\/\/[^\s<]+[^<.,:;"')\]\s])"#).unwrap()
});

#[derive(Debug, Modal)]
#[name = "Song Info"]
struct SongInfoModal {
	#[name = "URL Index"]
	#[placeholder = "The index of the URL to use. (default: 0)"]
	url_index: Option<String>,
	#[placeholder = "Whether or not to show the message."]
	ephemeral: Option<String>,
}

// Shows song information from a given link.
#[poise::command(
	context_menu_command = "Song Info",
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn song_info_context_menu(
	ctx: ApplicationContext<'_>,
	#[description = "The message to get the link from."] message: Message,
) -> Result<()> {
	let data = SongInfoModal::execute(ctx)
		.await?
		.ok_or_else(|| anyhow!("No modal data."))?;

	let ephemeral = match data.ephemeral.as_deref() {
		Some("false") => false,
		Some(_) => true,
		None => false,
	};

	let urls = LINK_REGEX
		.find_iter(&message.content)
		.map(|m| Url::parse(m.as_str()))
		.collect::<Vec<_>>();
	let url_index = match data.url_index.as_deref() {
		Some(url_index) => url_index.parse::<usize>()?,
		None => 0,
	};
	let url = urls.get(url_index).ok_or_else(|| {
		anyhow!(
			"You chose URL {} but there {} only {} URL{}.",
			url_index + 1,
			if urls.len() == 1 { "is" } else { "are" },
			urls.len(),
			if urls.len() == 1 { "" } else { "s" },
		)
	})?;
	let url = url
		.clone()
		.map_err(|_| anyhow!("Somehow the selected URL is invalid."))?;

	let mut youtube_downloader = YouTubeDownloader::builder().build();

	let (reply, disabled_reply) = build_song_info_message(
		&Context::Application(ctx),
		url,
		None,
		None,
		&mut youtube_downloader,
		ephemeral,
	)
	.await?;

	let message = ctx.send(reply).await?;
	song_interactions(
		&Context::Application(ctx),
		&message,
		&disabled_reply,
		youtube_downloader,
	)
	.await?;
	message
		.edit(poise::Context::Application(ctx), disabled_reply)
		.await?;

	Ok(())
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
	#[description = "The link to the song to look up."] url: Option<Url>,
	#[description = "The search query to look up the song with."]
	query: Option<String>,
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

	let mut youtube_downloader = YouTubeDownloader::builder().build();

	let (reply, disabled_reply) = match (url, query) {
		(Some(url), None) => {
			build_song_info_message(
				&ctx,
				url,
				None,
				None,
				&mut youtube_downloader,
				ephemeral,
			)
			.await?
		}
		(None, Some(query)) => {
			build_song_info_message(
				&ctx,
				get_song_link_searchable_link(query).await?,
				None,
				None,
				&mut youtube_downloader,
				ephemeral,
			)
			.await?
		}
		(None, None) => {
			return Err(anyhow!("No URL or query provided."));
		}
		(Some(_), Some(_)) => {
			return Err(anyhow!("Only provide a URL or a query, not both."));
		}
	};

	let message = ctx.send(reply).await?;
	song_interactions(&ctx, &message, &disabled_reply, youtube_downloader)
		.await?;
	message.edit(ctx, disabled_reply).await?;

	Ok(())
}

pub async fn song_interactions<'a>(
	ctx: &Context<'a>,
	message: &ReplyHandle<'a>,
	disabled_reply: &CreateReply<'a>,
	mut youtube_downloader: YouTubeDownloader,
) -> Result<()> {
	while match ComponentInteractionCollector::new(&ctx.serenity_context())
		.message_id(message.message().await?.id)
		.timeout(Duration::from_secs(60 * 5))
		.await
	{
		Some(ref interaction) => {
			interaction
				.create_response(
					ctx.http(),
					CreateInteractionResponse::Defer(
						CreateInteractionResponseMessage::new(),
					),
				)
				.await?;

			message.edit(ctx.clone(), disabled_reply.clone()).await?;

			let mut audio = CreateInteractionResponseFollowup::new();
			if youtube_downloader.was_started() {
				let file_name = youtube_downloader.get_file_name();
				let error = youtube_downloader.get_error();
				if let Ok(file_bytes) = youtube_downloader.wait().await {
					audio = audio.add_file(CreateAttachment::bytes(
						file_bytes, file_name,
					));
				} else if let Some(error) = error {
					audio = audio.content_or_attachment(|is_content| {
						if is_content {
							format!("```\n", error:#, "\n```")
						} else {
							format!(error:#)
						}
					})
				}
			};
			interaction.create_followup(ctx.http(), audio).await?;

			false
		}
		None => false,
	} {}

	Ok(())
}
