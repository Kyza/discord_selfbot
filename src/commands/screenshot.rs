use favicon_picker::get_favicons_from_url;
use resvg::{tiny_skia, usvg};
use thirtyfour::prelude::*;
use url::Url;

use crate::{
	config::Context,
	helpers::{colour_from_image, scale_to_min},
};
use anyhow::Result;
use inline_format::{format, println};
use poise::{
	serenity_prelude::{
		CreateAllowedMentions, CreateAttachment, CreateComponent,
		CreateContainer, CreateMediaGallery, CreateMediaGalleryItem,
		CreateSection, CreateSectionAccessory, CreateSectionComponent,
		CreateTextDisplay, CreateThumbnail, CreateUnfurledMediaItem,
		MessageFlags,
	},
	CreateReply,
};

// TODO: Create context menu version.

/// Screenshots a website.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn screenshot(
	ctx: Context<'_>,
	#[description = "The URL to screenshot."] url: Url,
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

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.flags(MessageFlags::IS_COMPONENTS_V2)
		.ephemeral(ephemeral);

	let screenshot = screenshot_url(&url).await?;
	let favicon = get_favicon_from_url(&url, 0).await;

	let mut title = CreateComponent::TextDisplay(CreateTextDisplay::new(
		format!("# ", url).to_owned(),
	));
	if let Some(favicon) = favicon.clone() {
		let section_title = CreateSectionComponent::TextDisplay(
			CreateTextDisplay::new(format!("## ", url).to_owned()),
		);
		title = CreateComponent::Section(CreateSection::new(
			vec![section_title].to_owned(),
			CreateSectionAccessory::Thumbnail(CreateThumbnail::new(
				CreateUnfurledMediaItem::new("attachment://favicon.png"),
			)),
		));
		reply =
			reply.attachment(CreateAttachment::bytes(favicon, "favicon.png"));
	}
	let image = CreateComponent::MediaGallery(CreateMediaGallery::new(
		vec![CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
			"attachment://screenshot.png",
		))]
		.to_owned(),
	));
	let container = CreateComponent::Container(
		CreateContainer::new(vec![title, image].to_owned()).accent_color(
			colour_from_image(
				&ctx,
				&if let Some(favicon) = favicon {
					favicon
				} else {
					screenshot.clone()
				},
			)?,
		),
	);
	reply = reply.components(vec![container].to_owned());
	reply = reply
		.attachment(CreateAttachment::bytes(screenshot, "screenshot.png"));

	ctx.send(reply).await?;

	Ok(())
}

pub async fn get_favicon_from_url(
	url: &Url,
	index: usize,
) -> Option<Vec<u8>> {
	let client = reqwest::Client::new();
	let favicons = get_favicons_from_url(&client, url).await.ok()?;
	// println!(favicons:#?);
	let favicon = favicons.iter().nth(index);
	if let Some(favicon) = favicon {
		let favicon_bytes =
			favicon.get_image_bytes(&client).await.ok()?.to_vec();
		match favicon.type_.as_deref() {
			Some("image/svg+xml") => {
				let tree = usvg::Tree::from_data(
					&favicon_bytes,
					&usvg::Options::default(),
				)
				.ok()?;
				let pixmap_size = tree.size().to_int_size();
				let (scaled_width, scaled_height, scale) = scale_to_min(
					pixmap_size.width(),
					pixmap_size.height(),
					1024,
				);
				let mut pixmap =
					tiny_skia::Pixmap::new(scaled_width, scaled_height)?;
				let transform =
					usvg::Transform::default().pre_scale(scale, scale);
				resvg::render(&tree, transform, &mut pixmap.as_mut());
				let png = pixmap.encode_png().ok()?.to_vec();
				Some(png)
			}
			_ => Some(favicon_bytes),
		}
	} else {
		None
	}
}

pub async fn screenshot_url(url: &Url) -> Result<Vec<u8>> {
	println!("Starting Firefox.");

	let mut caps = DesiredCapabilities::firefox();
	// If in debug mode, run non-headless.
	// Look at him he has Smitty Werbenjägermanjensen's hat.
	if !cfg!(debug_assertions) {
		caps.set_headless()?;
	}
	let driver = WebDriver::new("http://localhost:4444", caps).await?;

	driver.goto(url.to_string()).await?;

	// Wait for the page to load.
	let page_png = driver.screenshot_as_png().await?;

	Ok(page_png)
}

// async fn start_geckodriver() -> Result<Child> {
// 	let mut geckodriver = process::Command::new("geckodriver")
// 		.arg("--port=4444")
// 		.stdout(process::Stdio::piped())
// 		.stderr(process::Stdio::piped())
// 		.spawn()
// 		.expect("Failed to start geckodriver");

// 	// Get handles to stdout and stderr
// 	let stdout = geckodriver.stdout.take().unwrap();
// 	let stderr = geckodriver.stderr.take().unwrap();

// 	// Create readers
// 	let stdout_reader = std::io::BufReader::new(stdout);
// 	let stderr_reader = std::io::BufReader::new(stderr);

// 	// Wait for any output (either from stdout or stderr)
// 	use std::io::BufRead;
// 	let (tx, rx) = std::sync::mpsc::channel();

// 	// Monitor stdout
// 	let tx_stdout = tx.clone();
// 	std::thread::spawn(move || {
// 		if let Some(line) = stdout_reader.lines().next() {
// 			if let Ok(line) = line {
// 				tx_stdout.send(line).ok();
// 			}
// 		}
// 	});

// 	// Monitor stderr
// 	std::thread::spawn(move || {
// 		if let Some(line) = stderr_reader.lines().next() {
// 			if let Ok(line) = line {
// 				tx.send(line).ok();
// 			}
// 		}
// 	});

// 	// Wait for the first line of output
// 	let a = rx.recv().expect("Failed to get geckodriver output");
// 	println!("geckodriver output: {}", a);

// 	// Sleep for a second to ensure it's ready.
// 	tokio::time::sleep(Duration::from_millis(1000)).await;

// 	Ok(geckodriver)
// }
