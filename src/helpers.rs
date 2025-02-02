use std::{
	ffi::OsStr,
	fs::{self},
	path::{Path, PathBuf},
	time::Duration,
};

use anyhow::{anyhow, Result};
use poise::{
	serenity_prelude::{Attachment, CreateAttachment, EmbedThumbnail},
	CreateReply,
};
use reqwest::header;

#[macro_export]
macro_rules! crunch {
	($($name:ident),* $(,)?) => {
		$(
			mod $name;
			#[allow(ambiguous_glob_reexports)]
			pub use $name::*;
		)*
	};
}
pub use crunch;
use thirtyfour::{prelude::ElementQueryable, By, WebDriver, WebElement};

pub trait CreateReplyExt {
	fn content_or_attachment<F>(&self, cb: F) -> Self
	where
		F: Fn(bool) -> String;
}
impl CreateReplyExt for CreateReply {
	fn content_or_attachment<F>(&self, cb: F) -> Self
	where
		F: Fn(bool) -> String,
	{
		let content_text = cb(true);
		if content_text.len() <= 2000 {
			self.clone().content(content_text)
		} else {
			let attachment_text = cb(false);
			self.clone().attachment(CreateAttachment::bytes(
				attachment_text.as_bytes(),
				"text.txt",
			))
		}
	}
}

pub fn easy_set_file_name(path: &str, name: &str) -> Box<str> {
	let pathified = Path::new(path);
	pathified
		.with_file_name(name)
		.with_extension(pathified.extension().unwrap_or(OsStr::new("png")))
		.to_str()
		.unwrap_or(path)
		.into()
}

pub async fn wait_for_element(
	driver: &WebDriver,
	selector: &str,
) -> Result<WebElement> {
	// Sometimes the element becomes stale instantly. No idea why. Cry about it.
	while let Err(_) = driver
		.query(By::Css(selector))
		.wait(Duration::from_secs(60), Duration::from_millis(10))
		.first()
		.await
	{}
	Ok(driver
		.query(By::Css(selector))
		.wait(Duration::from_secs(60), Duration::from_millis(10))
		.first()
		.await?)
}

pub fn safe_delete(path: &PathBuf) -> Result<bool> {
	if fs::exists(path)? {
		fs::remove_file(path)?;
		Ok(true)
	} else {
		Ok(false)
	}
}

#[derive(Debug, Clone)]
pub enum AttachmentOrThumbnail {
	Attachment(Attachment),
	Embed(EmbedThumbnail),
}
impl AttachmentOrThumbnail {
	pub async fn download(
		&self,
		client: &reqwest::Client,
	) -> Result<Vec<u8>> {
		match self {
			AttachmentOrThumbnail::Attachment(a) => Ok(a.download().await?),
			AttachmentOrThumbnail::Embed(e) => {
				// Download the image from the proxy URL.
				let url = e.proxy_url.as_ref().ok_or_else(|| {
					anyhow!("Embed thumbnail has no proxy URL")
				})?;
				let request = client.get(url).send().await?;
				Ok(request.bytes().await?.to_vec())
			}
		}
	}

	pub fn filename(&self) -> String {
		match self {
			AttachmentOrThumbnail::Attachment(a) => a.filename.clone(),
			AttachmentOrThumbnail::Embed(e) => {
				// Parse the URL to get the filename.
				let url = &e.proxy_url;
				let url = if let Some(url) = url {
					url::Url::parse(&url).unwrap_or_else(|_| {
						url::Url::parse("https://example.com/thumbnail.png")
							.unwrap()
					})
				} else {
					return "thumbnail.png".to_string();
				};
				if let Some(path_segments) = url.path_segments() {
					let filename = path_segments
						.last()
						.unwrap_or_else(|| "thumbnail.png");
					filename.to_string()
				} else {
					"thumbnail.png".to_string()
				}
			}
		}
	}
}

pub async fn is_file_larger_than_mb(
	url: &str,
	max_size_mb: u64,
) -> Result<(bool, u64)> {
	let client = reqwest::Client::new();
	let response = client.head(url).send().await?;

	println!("{:#?}", response.headers());

	if let Some(content_length) =
		response.headers().get(header::CONTENT_LENGTH)
	{
		if let Ok(size) =
			content_length.to_str().unwrap_or("0").parse::<u64>()
		{
			Ok((size > max_size_mb * 1024 * 1024, size))
		} else {
			Err(anyhow!("Could not parse content-length header"))
		}
	} else {
		Err(anyhow!("Content-Length header not found"))
	}
}

pub fn change_extension<P: AsRef<Path>>(path: P, new_ext: &str) -> PathBuf {
	let mut new_path = path.as_ref().to_path_buf();

	// Remove the current extension (if any)
	new_path.set_extension(new_ext);

	new_path
}

pub fn escape_markdown(text: &str) -> String {
	let mut escaped = String::new();
	for c in text.chars() {
		// args.text.replace(/(`|\*|_|>|<)/g, "\\$1")
		match c {
			'`' => escaped.push_str("\\`"),
			'*' => escaped.push_str("\\*"),
			'_' => escaped.push_str("\\_"),
			'<' => escaped.push_str("\\<"),
			'>' => escaped.push_str("\\>"),
			'\\' => escaped.push_str("\\\\"),
			_ => escaped.push(c),
		}
	}
	escaped
}
