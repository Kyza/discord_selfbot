use anyhow::{anyhow, Result};
use byte_unit::rust_decimal::prelude::ToPrimitive;
use inline_format::{format, println};
use serde::Deserialize;
use url::Url;

use crate::{
	commands::{
		build_song_info_message, get_song_link_searchable_link,
		song_interactions,
	},
	config::Context,
	youtube_downloader::{DownloadFormat, YouTubeDownloader},
};

#[derive(Debug, Clone)]
pub struct PlayingNow {
	pub user_id: String,
	pub playing_now: bool,
	pub duration: Option<u32>,
	pub origin_url: Option<String>,
	pub artist_name: Option<String>,
	pub track_name: Option<String>,
}

impl<'de> Deserialize<'de> for PlayingNow {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value =
			serde_json::Value::deserialize(deserializer)?["payload"].clone();

		Ok(Self {
			user_id: value["user_id"].as_str().unwrap().to_string(),
			playing_now: value["playing_now"].as_bool().unwrap(),
			duration: value["listens"][0]["track_metadata"]
				["additional_info"]["duration"]
				.as_u64()
				.and_then(|s| s.to_u32()),
			origin_url: value["listens"][0]["track_metadata"]
				["additional_info"]["origin_url"]
				.as_str()
				.map(|s| s.to_string()),
			artist_name: value["listens"][0]["track_metadata"]["artist_name"]
				.as_str()
				.map(|s| s.to_string()),
			track_name: value["listens"][0]["track_metadata"]["track_name"]
				.as_str()
				.map(|s| s.to_string()),
		})
	}
}

/// Shows what you're currently listening to from the ListenBrainz API.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn now_playing(
	ctx: Context<'_>,
	#[description = "The user to show the currently playing song of."]
	user: Option<String>,
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

	let playing_now_data = ctx
		.data()
		.http
		.get(format!(
			"https://api.listenbrainz.org/1/user/",
			&urlencoding::encode(
				&user.or(
					ctx.data().config.listenbrainz_user.clone()
				).ok_or(
					anyhow!("No user was provided and no default user is set in the config.")
				)?
			),
			"/playing-now"
		))
		.send()
		.await?
		.json::<PlayingNow>()
		.await?;

	println!(playing_now_data:#?);

	let mut youtube_downloader = YouTubeDownloader::builder()
		.format(DownloadFormat::Audio)
		.build();

	let url = if let Some(playing_now_data_url) = playing_now_data.origin_url
	{
		let url = Url::parse(&playing_now_data_url)?;
		// Only download if the URL is a YouTube Music URL.
		// Sometimes YouTube doesn't serve audio only.
		if YouTubeDownloader::is_youtube_music_url(&url) {
			youtube_downloader.url = Some(url.clone());
			let _ = youtube_downloader.start_download();
		}
		url
	} else {
		let search_query = match (
			playing_now_data.track_name.clone(),
			playing_now_data.artist_name.clone(),
		) {
			(Some(track_name), Some(artist_name)) => {
				format!(track_name, " ", artist_name)
			}
			(Some(track_name), None) => track_name,
			(None, Some(artist_name)) => artist_name,
			_ => Err(anyhow!("No song is currently playing."))?,
		};
		get_song_link_searchable_link(search_query).await?
	};

	let (reply, disabled_reply) = build_song_info_message(
		&ctx,
		url,
		playing_now_data.track_name,
		playing_now_data.artist_name,
		&mut youtube_downloader,
		ephemeral,
	)
	.await?;

	let message = ctx.send(reply).await?;
	song_interactions(&ctx, &message, &disabled_reply, youtube_downloader)
		.await?;
	message.edit(ctx, disabled_reply).await?;

	Ok(())
}
