use anyhow::{anyhow, Result};
use poise::{
	serenity_prelude::{
		CreateActionRow, CreateAllowedMentions, CreateAttachment,
		CreateButton,
	},
	CreateReply,
};

// const EMBED_COLOR: u32 = 0xff6600;

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
	   "https://api.wolframalpha.com/v2/query?input={}&format=plaintext,image&reinterpret=true&mag=2&output=JSON&appid={}",
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

	let simple_results_api_url = format!(
	     "https://api.wolframalpha.com/v1/simple?input={}&reinterpret=true&mag=2&appid={}",
	     query,
	     ctx.data().wolfram_alpha_simple_app_id
	  );

	let simple_response =
		ctx.data().http.get(simple_results_api_url).send().await?;

	if !simple_response.status().is_success() {
		return Err(anyhow!(
			"Failed to get response from Wolfram Alpha Simple API."
		));
	}

	let image = simple_response.bytes().await?;

	let json: serde_json::Value = full_response.json().await?;
	let pods = json["queryresult"]["pods"]
		.as_array()
		.ok_or_else(|| anyhow!("Invalid response format"))?;

	println!("{:#?}", json.clone());

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.components(vec![CreateActionRow::Buttons(vec![
			CreateButton::new_link(format!(
				"https://www.wolframalpha.com/input?i={}",
				query
			))
			.label("View Online"),
		])])
		.ephemeral(ephemeral);

	let mut markdown = String::new();

	for pod in pods {
		let title = pod["title"].as_str().unwrap_or("Untitled");
		let subpods = pod["subpods"]
			.as_array()
			.ok_or_else(|| anyhow!("Invalid subpods format"))?;

		let mut has_plaintext = false;

		let mut plaintexts = Vec::new();

		for subpod in subpods {
			let plaintext = subpod["plaintext"].as_str();
			if let Some(plaintext) = plaintext {
				if !plaintext.is_empty() {
					has_plaintext = true;
					plaintexts.push(format!("```rs\n{}\n```", plaintext));
				}
			}
		}

		if has_plaintext {
			markdown.push_str(&format!("**__{}:__**", title));
			for plaintext in plaintexts {
				markdown.push_str(&plaintext);
			}
		}
	}

	if markdown.len() > 4000 {
		reply = reply.attachment(CreateAttachment::bytes(
			markdown,
			"wolfram_response.txt",
		));
	} else {
		reply = reply.content(markdown);
	}

	// let mut grid_items = Vec::new();
	// let mut grid_item_images = Vec::new();
	// for image in images {
	// 	if let Some(title) = image.0 {
	// 		grid_items.push(GridItem {
	// 			title,
	// 			images: grid_item_images,
	// 		});
	// 		grid_item_images = Vec::new();
	// 	}
	// 	grid_item_images.push(image.1);
	// }

	reply = reply
		.attachment(CreateAttachment::bytes(image, "wolfram_image.webp"));

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

// use resvg::tiny_skia::Pixmap;
// use std::io::Cursor;
// use usvg::{ImageRendering, Options, Transform, Tree};

// struct GridItem {
// 	title: String,
// 	images: Vec<Vec<u8>>, // Each image is stored as a byte vector
// }

// fn render_grid(
// 	items: Vec<GridItem>,
// 	img_width: u32,
// 	img_height: u32,
// 	padding: u32,
// 	border: u32,
// ) -> Result<Vec<u8>> {
// 	let cols = 3; // Number of columns in the grid
// 	let img_width_percent = 30.0; // Each image is 30% of the total width (adjust as needed)
// 	let img_height_percent = 30.0; // Adjust the height proportionally
// 	let padding = 0; // Padding between grid items (in percentage)
// 	let border = 0; // Border around images (in percentage)
// 	let svg_width = 100.0;
// 	let svg_height = 100.0;

// 	// SVG header
// 	let mut svg_content = format!(
// 		r#"<?xml version="1.0" standalone="no"?><svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" viewBox="0 0 {} {}">"#,
// 		svg_width, svg_height
// 	);

// 	// Calculate the number of rows needed
// 	let rows = (items.len() + cols - 1) / cols;
// 	let item_width_percent = 100.0 / cols as f32;
// 	let item_height_percent = 100.0 / rows as f32;

// 	let mut current_x_percent = 0.0;
// 	let mut current_y_percent = 0.0;

// 	// Add images and titles to the grid, grouping each set of images with its title
// 	for (i, item) in items.iter().enumerate() {
// 		// Open the group tag for each item
// 		svg_content.push_str(&format!(
// 			r#"<g transform="translate({}%, {}%)">"#,
// 			current_x_percent, current_y_percent
// 		));

// 		// Add the title as text in the group
// 		svg_content.push_str(&format!(
// 			r#"<text x="{}%" y="5%" font-size="3%" font-family="Arial">{}</text>"#,
// 			item_width_percent / 2.0,
// 			item.title
// 		));

// 		// Add each image inside the group
// 		for (j, img_data) in item.images.iter().enumerate() {
// 			let base64_img = base64::encode(&img_data);
// 			let img_str = format!("data:image/png;base64,{}", base64_img);
// 			let img_x = (j as f64)
// 				* (img_width_percent + padding as f64 + border as f64);
// 			let img_y = 10.0; // Position images below the title

// 			// Add the image to the group
// 			svg_content.push_str(&format!(
//                r#"<image href="{}" x="{}%" y="{}%" width="{}%" height="{}%" style="border:{}% solid black;" />"#,
//                img_str, img_x, img_y, img_width_percent, img_height_percent, border
//            ));
// 		}

// 		// Close the group tag
// 		svg_content.push_str("</g>");

// 		// Update position for the next item in the grid
// 		current_x_percent += item_width_percent;
// 		if current_x_percent >= 100.0 {
// 			current_x_percent = 0.0;
// 			current_y_percent += item_height_percent;
// 		}
// 	}

// 	// Close the SVG tag
// 	svg_content.push_str("</svg>");

// 	println!("{}", svg_content);

// 	// Parse the SVG document
// 	let tree = Tree::from_data(svg_content.as_bytes(), &Options::default())?;

// 	// Create a pixmap for rendering
// 	let mut pixmap =
// 		Pixmap::new(1000, 1000).ok_or(anyhow!("Failed to create pixmap"))?;

// 	// Render the SVG to the pixmap
// 	resvg::render(&tree, Transform::default(), &mut pixmap.as_mut());

// 	// Save the rendered image in memory
// 	let buffer = pixmap.encode_png()?;

// 	Ok(buffer) // Return the rendered image as a byte vector
// }

// fn combine_images(images: Vec<(Option<String>, RgbaImage)>) -> RgbaImage {
// 	let text_height = 36;
// 	let border_size = 2; // Size of the border

// 	// Filter out None titles and calculate the number of titles
// 	let title_count =
// 		images.iter().filter(|(title, _)| title.is_some()).count();

// 	// Determine the number of columns based on the number of titles
// 	let columns = match title_count {
// 		0..=1 => 1, // 1 column for 0 or 1 title
// 		2..=4 => 2, // 2 columns for 2 to 4 titles
// 		5..=8 => 3, // 3 columns for 5 to 8 titles
// 		_ => 4,     // 4 columns for more than 8 titles
// 	};

// 	// Calculate the number of rows needed based on the number of images and columns
// 	let total_images = images.len() as u32;
// 	let rows = (total_images + columns - 1) / columns; // Ceiling division

// 	// Initialize vectors to store max dimensions for each column
// 	let mut max_widths = vec![0; columns as usize];
// 	let mut max_heights = vec![0; rows as usize];

// 	// Determine max width and height for each column and row
// 	for (i, (_, img)) in images.iter().enumerate() {
// 		let col = i as u32 % columns; // Calculate current column
// 		let row = i as u32 / columns; // Calculate current row

// 		// Update the max dimensions for the current column and row
// 		max_widths[col as usize] = max_widths[col as usize].max(img.width());
// 		max_heights[row as usize] =
// 			max_heights[row as usize].max(img.height());
// 	}

// 	// Calculate total height based on max heights and text height
// 	let total_height: u32 =
// 		max_heights.iter().map(|&h| h + text_height).sum::<u32>()
// 			+ (rows * border_size * 2);

// 	// Total width is based on the sum of max widths + borders
// 	let total_width: u32 =
// 		max_widths.iter().sum::<u32>() + (columns * border_size * 2);

// 	// Create a new image with calculated width and height
// 	let mut combined_image = RgbaImage::new(total_width, total_height);

// 	// Fill the background with white
// 	let fill_rect = Rect::at(0, 0)
// 		.of_size(combined_image.width(), combined_image.height());
// 	draw_filled_rect_mut(
// 		&mut combined_image,
// 		fill_rect,
// 		Rgba([255, 255, 255, 255]),
// 	);

// 	// Font loading for titles (from the assets folder)
// 	let font_data = include_bytes!(concat!(
// 		env!("CARGO_MANIFEST_DIR"),
// 		"/assets/Roboto-Regular.ttf"
// 	));
// 	let font =
// 		FontRef::try_from_slice(font_data).expect("Error loading font");
// 	let scale = PxScale {
// 		x: text_height as f32,
// 		y: text_height as f32,
// 	};

// 	// Overlay the images in a grid format
// 	for (i, (title, img)) in images.into_iter().enumerate() {
// 		let row = i as u32 / columns; // Calculate current row
// 		let col = i as u32 % columns; // Calculate current column

// 		let x_offset = (col as u32)
// 			* (max_widths[col as usize] + border_size)
// 			+ border_size; // X position for the current image
// 		let y_offset = (max_heights.iter().take(row as usize).sum::<u32>()
// 			+ text_height * row
// 			+ border_size * (row + 1))
// 			+ (border_size * row); // Y position for the current image

// 		// Draw borders around the image and title
// 		let border_color = Rgba([0, 0, 0, 255]); // Border color (black)
// 		let border_rect = Rect::at(0, y_offset as i32 - border_size as i32)
// 			.of_size(combined_image.width(), border_size);
// 		draw_filled_rect_mut(&mut combined_image, border_rect, border_color);

// 		// Draw left border
// 		let left_border_rect =
// 			Rect::at(x_offset as i32 - border_size as i32, 0)
// 				.of_size(border_size, combined_image.height());
// 		draw_filled_rect_mut(
// 			&mut combined_image,
// 			left_border_rect,
// 			border_color,
// 		);

// 		// If a title exists, draw it
// 		if let Some(title_text) = title {
// 			let title_x = x_offset; // Center the title with a small margin
// 			let title_y = y_offset; // Title drawn at the top of the cell
// 			draw_text_mut(
// 				&mut combined_image,
// 				Rgba([0, 0, 0, 255]),
// 				title_x as i32,
// 				title_y as i32,
// 				scale,
// 				&font,
// 				&title_text,
// 			);
// 		}

// 		// Overlay the image below the title (or directly if no title)
// 		image::imageops::overlay(
// 			&mut combined_image,
// 			&img,
// 			x_offset.into(),
// 			(y_offset + text_height + border_size).into(),
// 		);
// 	}

// 	combined_image
// }

// fn rgba_to_vec(img: RgbaImage) -> Result<Vec<u8>, image::ImageError> {
// 	let mut bytes: Vec<u8> = Vec::new();
// 	img.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::WebP)?;
// 	Ok(bytes)
// }
