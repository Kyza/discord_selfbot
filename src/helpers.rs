use std::{
	ffi::OsStr,
	fs::{self},
	path::{Path, PathBuf},
	time::Duration,
};

use anyhow::{anyhow, Result};
use poise::{
	serenity_prelude::{
		Attachment, Colour, CreateAttachment,
		CreateInteractionResponseFollowup, EmbedThumbnail,
	},
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

use crate::config::{Color, Context};

pub trait ContentOrAttachmentExt {
	fn content_or_attachment<F>(&self, cb: F) -> Self
	where
		F: Fn(bool) -> String;
}
impl<'a> ContentOrAttachmentExt for CreateReply<'a> {
	fn content_or_attachment<F>(&self, cb: F) -> Self
	where
		F: Fn(bool) -> String,
	{
		let content_text = cb(true);
		if content_text.len() <= 2000 {
			self.clone().content(content_text)
		} else {
			let attachment_text = cb(false).clone();
			let attachment_text = attachment_text.as_bytes().to_owned();
			self.clone().attachment(CreateAttachment::bytes(
				attachment_text,
				"text.txt",
			))
		}
	}
}
impl<'a> ContentOrAttachmentExt for CreateInteractionResponseFollowup<'a> {
	fn content_or_attachment<F>(&self, cb: F) -> Self
	where
		F: Fn(bool) -> String,
	{
		let content_text = cb(true);
		if content_text.len() <= 2000 {
			self.clone().content(content_text)
		} else {
			let attachment_text = cb(false).clone();
			let attachment_text = attachment_text.as_bytes().to_owned();
			self.clone().add_file(CreateAttachment::bytes(
				attachment_text,
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
	while driver
		.query(By::Css(selector))
		.wait(Duration::from_secs(60), Duration::from_millis(10))
		.first()
		.await
		.is_err()
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
				let request = client.get(url.to_string()).send().await?;
				Ok(request.bytes().await?.to_vec())
			}
		}
	}

	pub fn filename(&self) -> String {
		match self {
			AttachmentOrThumbnail::Attachment(a) => a.filename.to_string(),
			AttachmentOrThumbnail::Embed(e) => {
				// Parse the URL to get the filename.
				let url = &e.proxy_url;
				let url = if let Some(url) = url {
					url::Url::parse(url).unwrap_or_else(|_| {
						url::Url::parse("https://example.com/thumbnail.png")
							.unwrap()
					})
				} else {
					return "thumbnail.png".to_string();
				};
				if let Some(path_segments) = url.path_segments() {
					let filename =
						path_segments.last().unwrap_or("thumbnail.png");
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

pub fn colour_from_image(
	ctx: &Context,
	image_bytes: &Vec<u8>,
) -> Result<Colour> {
	use color_thief::{get_palette, ColorFormat};

	let color_bytes = image::load_from_memory(&image_bytes)
		.unwrap()
		.to_rgb8()
		.into_raw();

	Ok(get_palette(&color_bytes[..], ColorFormat::Rgb, 10, 2)?
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
		.into())
}

/// Ensures the width or the height is at least `min_width_or_height` and scales the other.
pub fn scale_to_min(
	width: u32,
	height: u32,
	min_width_or_height: u32,
) -> (u32, u32, f32) {
	let scale = min_width_or_height as f32 / width.max(height) as f32;
	let new_width = (width as f32 * scale) as u32;
	let new_height = (height as f32 * scale) as u32;
	(new_width, new_height, scale)
}
