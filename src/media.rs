use core::str;
use std::io::{Read, Write};
use std::process::Command;
use std::{fs::File, path::Path};

use anyhow::{anyhow, Result};
use byte_unit::Byte;
use tempfile::NamedTempFile;

#[derive(Debug)]
pub struct CompressedFile {
	pub file: NamedTempFile,
	pub scale: f64,
	pub quality: u32,
}

pub fn compress_file(input: &Path) -> Result<(NamedTempFile, String)> {
	let media_type = determine_media_type(input);

	println!("Media type: {:?}", media_type);

	match media_type {
		MediaType::Image => {
			Ok((compress_image(input, 0)?, "webp".to_string()))
		}
		MediaType::Video => {
			Ok((compress_video(input, 0)?, "mp4".to_string()))
		}
		// MediaType::Audio => {
		// 	Ok((compress_audio(input, 0)?, "mp3".to_string()))
		// }
		MediaType::Unknown | _ => Err(anyhow!("Unsupported file type.")),
	}
}

#[derive(Debug)]
enum MediaType {
	Image,
	Video,
	// Audio,
	Unknown,
}

fn determine_media_type(file_path: &Path) -> MediaType {
	let output = Command::new("ffprobe")
		.args([
			"-v",
			"error",
			"-show_entries",
			"format=format_name",
			"-of",
			"default=noprint_wrappers=1:nokey=1",
			file_path.to_str().unwrap(),
		])
		.output();

	if let Ok(output) = output {
		if output.status.success() {
			if let Ok(format) = str::from_utf8(&output.stdout) {
				let format = format.trim();

				println!("Format: {}", format);

				// List of known image formats
				let image_formats = [
					"jpeg", "jpg", "png", "bmp", "gif", "tiff", "webp",
					"jxl", "heic", "heif", "avif", "svg",
				];

				// List of known video formats
				let video_formats = [
					"mp4", "mkv", "mov", "avi", "flv", "wmv", "webm", "mpeg",
				];

				if image_formats.iter().any(|&f| format.contains(f)) {
					return MediaType::Image;
				} else if video_formats.iter().any(|&f| format.contains(f)) {
					return MediaType::Video;
				}
			}
		}
	}

	MediaType::Unknown
}

const TARGET_SIZE_MB: u64 = 8;
fn mb_to_bytes(mb: u64) -> u64 {
	mb * 1024 * 1024
}

fn compress_video(input: &Path, attempt: u8) -> Result<NamedTempFile> {
	if !input.exists() {
		return Err(anyhow!(
			"Input file does not exist: {}",
			input.display()
		));
	}
	if attempt >= 2 {
		return Err(anyhow!("Ran out of attempts while compressing video."));
	}

	let mut input_file = File::open(input)?;

	let target_size_adjusted =
		mb_to_bytes(TARGET_SIZE_MB - attempt as u64 - 1);

	println!(
		"Target size adjusted: {}",
		Byte::from_u64(target_size_adjusted)
	);

	// If the file's already small enough, don't bother converting it.
	if input_file.metadata()?.len() <= target_size_adjusted {
		println!("File is already small enough.");
		return file_to_named_temp_file(&mut input_file);
	}

	let input_path = input.to_str().unwrap();

	let output = NamedTempFile::new()?;
	let output_path = output.path().to_str().unwrap();

	println!("Converting video.");

	let mut ffmpeg_command = Command::new("ffmpeg");

	ffmpeg_command.args([
		"-y", "-i", input_path, "-c:v", "libx265", "-preset", "medium", "-f",
		"mp4",
	]);

	println!("Compressing video.");
	let video_info = get_video_info(input)?;

	let new_bitrate = estimate_video_bitrate(
		video_info.video_bitrate,
		video_info.audio_bitrate,
		video_info.duration,
		target_size_adjusted * 8, // Convert to bits
	);
	if new_bitrate < video_info.video_bitrate {
		println!("Reducing FPS to 30.");
		ffmpeg_command.args(["-vf", &format!("fps={}", 30)]);
	}

	let new_bitrate = format!("{}k", new_bitrate / 1024);

	println!(
		"Old bitrate: {}",
		format!("{}k", video_info.video_bitrate / 1024)
	);
	println!("New bitrate: {}", new_bitrate);

	ffmpeg_command.args(["-b:v", &new_bitrate]);

	ffmpeg_command.arg(output_path);

	let ffmpeg_output = ffmpeg_command
		.output()
		.map_err(|e| anyhow!("Failed to execute FFmpeg: {}", e))?;

	if !ffmpeg_output.status.success() {
		return Err(anyhow!(
			"Compression failed: {}",
			String::from_utf8_lossy(&ffmpeg_output.stderr)
		));
	}

	let output_file_size = output.as_file().metadata()?.len();
	let target_size = mb_to_bytes(TARGET_SIZE_MB);
	if output_file_size > target_size {
		return compress_video(input, attempt + 1);
	}

	Ok(output)
}

fn compress_image(input: &Path, attempt: u8) -> Result<NamedTempFile> {
	if !input.exists() {
		return Err(anyhow!(
			"Input file does not exist: {}",
			input.display()
		));
	}
	if attempt >= 3 {
		return Err(anyhow!("Ran out of attempts while compressing image."));
	}

	let input_path = input.to_str().unwrap();

	let output = tempfile::NamedTempFile::new()?;
	let output_path = output.path().to_str().unwrap();

	// Ensure quality is between 0 and 100
	let max_fps = 30;

	println!("Converting image.");

	let mut ffmpeg_command = Command::new("ffmpeg");

	ffmpeg_command.args([
		"-y",
		"-i",
		input_path,
		"-vf",
		&format!("fps={}", max_fps),
		"-vcodec",
		"libwebp",
		"-lossless",
		"1",
		"-compression_level",
		"6",
		"-loop",
		"0",
		"-preset",
		"picture",
		"-an",
		"-vsync",
		"vfr",
		"-f",
		"webp",
	]);

	println!("Compressing image.");
	ffmpeg_command.args(["-q:v", &(90 - (attempt as u64 * 5)).to_string()]);

	ffmpeg_command.arg(output_path);

	let ffmpeg_output = ffmpeg_command
		.output()
		.map_err(|e| anyhow!("Failed to execute FFmpeg: {}", e))?;

	if !ffmpeg_output.status.success() {
		return Err(anyhow!(
			"Compression failed: {}",
			String::from_utf8_lossy(&ffmpeg_output.stderr)
		));
	}

	let output_file_size = output.as_file().metadata()?.len();
	let target_size = mb_to_bytes(TARGET_SIZE_MB);
	if output_file_size > target_size {
		return compress_image(input, attempt + 1);
	}

	Ok(output)
}

pub fn file_to_named_temp_file(file: &mut File) -> Result<NamedTempFile> {
	let mut tempfile = NamedTempFile::new()?;
	let mut buffer = Vec::new();
	file.read_to_end(&mut buffer)?;
	tempfile.write_all(&buffer[..])?;
	Ok(tempfile)
}

pub fn estimate_media_size(
	video_bitrate: u64,
	audio_bitrate: u64,
	duration_seconds: f64,
) -> f64 {
	// Convert bitrate
	let video_bitrate_mbps = video_bitrate as f64 / 1024.0 / 1024.0;
	let audio_bitrate_mbps = audio_bitrate as f64 / 1024.0 / 1024.0;

	// Calculate file size in bytes
	(video_bitrate_mbps * duration_seconds) / 8.0
		+ (audio_bitrate_mbps * duration_seconds) / 8.0
}

pub fn estimate_video_bitrate(
	starting_bitrate: u64,
	audio_bitrate: u64,
	duration_seconds: f64,
	target_size: u64,
) -> u64 {
	let mut new_bitrate = starting_bitrate as f64;
	while (new_bitrate * duration_seconds)
		+ (audio_bitrate as f64 * duration_seconds)
		> target_size as f64
	{
		new_bitrate -= 1024.0;
	}
	new_bitrate as u64
}

#[derive(Debug)]
pub struct VideoInfo {
	pub video_bitrate: u64,
	pub audio_bitrate: u64,
	pub duration: f64,
}

pub fn get_video_info(file_path: &Path) -> Result<VideoInfo> {
	// Retrieve bitrate and resolution
	let duration = get_media_stream_info(file_path, "v:0", "duration")?
		.trim()
		.parse::<f64>()
		.unwrap();
	let video_bitrate = get_media_stream_info(file_path, "v:0", "bit_rate")?
		.trim()
		.parse::<u64>()
		.unwrap();
	let audio_bitrate = get_media_stream_info(file_path, "a:0", "bit_rate")?
		.trim()
		.parse::<u64>()
		.unwrap();

	Ok(VideoInfo {
		video_bitrate,
		audio_bitrate,
		duration,
	})
}

pub fn get_media_stream_info(
	file_path: &Path,
	stream: &str,
	entry: &str,
) -> Result<String> {
	// Execute the ffprobe command
	let output = Command::new("ffprobe")
		.arg("-v")
		.arg("error")
		.arg("-select_streams")
		.arg(stream)
		.arg("-show_entries")
		.arg(format!("stream={}", entry))
		.arg("-of")
		.arg("default=noprint_wrappers=1:nokey=1")
		.arg(file_path.to_str().unwrap())
		.output()
		.map_err(|e| anyhow!("Failed to execute ffprobe: {}", e))?;

	// Check if the command was successful
	if !output.status.success() {
		return Err(anyhow!(
			"ffprobe error: {}",
			str::from_utf8(&output.stderr).unwrap_or("unknown error")
		));
	}

	// Parse the output
	let output_str = str::from_utf8(&output.stdout)
		.map_err(|e| anyhow!("Failed to parse output: {}", e))?;

	Ok(output_str.to_string())
}
