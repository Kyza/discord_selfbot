use std::{fs, process, sync::Arc};

use anyhow::{anyhow, Error, Result};
use bon::bon;
use rand::Rng;
use tempfile::env;
use url::Url;

use crate::os_command::run_os_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadFormat {
	Video,
	Audio,
}

pub type DownloadFuture = tokio::task::JoinHandle<Result<Vec<u8>>>;

pub struct YouTubeDownloader {
	pub url: Option<Url>,
	pub format: DownloadFormat,
	file_bytes: Option<Vec<u8>>,
	downloading_future: Option<DownloadFuture>,
	error: Option<Arc<Error>>,
	file_name: String,
}

#[bon]
impl YouTubeDownloader {
	pub fn is_valid_url(url: &Url) -> bool {
		url.host_str().map_or(false, |host| match host {
			"youtube.com" | "www.youtube.com" | "youtu.be"
			| "www.youtu.be" | "music.youtube.com" => true,
			_ => false,
		})
	}
	pub fn is_youtube_url(url: &Url) -> bool {
		url.host_str().map_or(false, |host| match host {
			"youtube.com" | "www.youtube.com" | "youtu.be" => true,
			_ => false,
		})
	}
	pub fn is_youtube_music_url(url: &Url) -> bool {
		url.host_str().map_or(false, |host| match host {
			"www.youtu.be" | "music.youtube.com" => true,
			_ => false,
		})
	}

	#[builder]
	pub fn new(url: Option<Url>, format: Option<DownloadFormat>) -> Self {
		YouTubeDownloader {
			url,
			format: format.unwrap_or(DownloadFormat::Video),
			file_bytes: None,
			downloading_future: None,
			error: None,
			file_name: "unknown.mp3".to_string(),
		}
	}
	pub fn get_url(&self) -> Option<Url> {
		self.url.clone()
	}
	pub fn get_error(&self) -> Option<Arc<Error>> {
		self.error.clone()
	}
	pub fn get_file_name(&self) -> String {
		self.file_name.clone()
	}
	pub fn file_name(&mut self, file_name: impl ToString) {
		self.file_name = file_name.to_string();
	}

	/// Starts downloading the YouTube video from the given URL using yt-dlp asynchronously.
	/// The future is automatically saved on the struct to prevent it from being dropped prematurely.
	pub fn start_download(&mut self) -> Result<&Self> {
		if self.was_started() {
			return Ok(self);
		}

		let url =
			self.get_url().ok_or_else(|| anyhow!("No URL provided."))?;

		let format = self.format.clone();

		self.downloading_future = Some(tokio::spawn(async move {
			// Implementation of downloading logic.
			// Generate a random filename.
			let yt_dlp_output_path = &env::temp_dir()
				.join(rand::thread_rng().gen::<u64>().to_string())
				.to_string_lossy()
				.to_string();
			let format_code = match format {
				DownloadFormat::Audio => "bestaudio[filesize<=8M]",
				DownloadFormat::Video => "best[filesize<=8M]",
			};

			let mut yt_dlp_command = process::Command::new("yt-dlp");
			yt_dlp_command.args([
				url.as_str(),
				"-o",
				yt_dlp_output_path,
				"-f",
				format_code,
			]);
			let yt_dlp_output = run_os_command("yt-dlp", yt_dlp_command)?;

			if yt_dlp_output.status.success() {
				// Read the file.
				let data = fs::read(yt_dlp_output_path)?;

				if fs::exists(yt_dlp_output_path)? {
					fs::remove_file(yt_dlp_output_path)?;
				}

				Ok(data)
			} else {
				if fs::exists(yt_dlp_output_path)? {
					fs::remove_file(yt_dlp_output_path)?;
				}

				Err(anyhow!(
					"{}",
					String::from_utf8_lossy(yt_dlp_output.stderr.as_slice())
				))
			}
		}));

		Ok(self)
	}

	/// Waits for the download to complete.
	pub async fn wait(&mut self) -> Result<Vec<u8>> {
		if self.is_done() {
			return self.file_bytes();
		}

		if let Some(future) = self.downloading_future.take() {
			match future.await {
				Ok(Ok(data)) => {
					self.file_bytes = Some(data);
				}
				Ok(Err(err)) => {
					self.error = Some(Arc::new(err));
				}
				Err(err) => {
					self.error = Some(Arc::new(err.into()));
				}
			}
		}

		self.file_bytes()
	}

	pub fn file_bytes(&self) -> Result<Vec<u8>> {
		self.file_bytes
			.clone()
			.ok_or_else(|| anyhow!("Failed to download file."))
	}

	pub fn is_done(&self) -> bool {
		self.file_bytes.is_some()
	}

	pub fn is_downloading(&self) -> bool {
		self.downloading_future.is_some()
	}

	pub fn was_started(&self) -> bool {
		self.is_done() || self.is_downloading()
	}
}
