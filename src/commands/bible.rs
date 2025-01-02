use std::collections::HashSet;

use anyhow::{anyhow, Result};
use inline_format::format;
use poise::{
	serenity_prelude::{CreateAllowedMentions, Message},
	CreateReply, Modal,
};
use regex::Regex;

use crate::types::ApplicationContext;

const KJV_BIBLE: &str = include_str!("../../assets/kjv.txt");

#[derive(Debug)]
pub struct AnalysisResult {
	total_words: usize,
	matched_words: usize,
	match_percentage: f64,
	highlighted_text: String,
	// unmatched_words: HashSet<String>,
}

pub fn analyze_text(
	search_text: &str,
	target_text: &str,
) -> Result<AnalysisResult> {
	// Create regex for word boundaries
	let word_boundary = Regex::new(r"\b[\w'-]+\b")?;

	// Extract all words from search text and convert to lowercase
	let search_words: HashSet<String> = word_boundary
		.find_iter(search_text)
		.map(|m| m.as_str().to_lowercase())
		.collect();

	// Create set of words from target text
	let target_words: HashSet<String> = word_boundary
		.find_iter(target_text)
		.map(|m| m.as_str().to_lowercase())
		.collect();

	// Calculate unmatched words
	let unmatched_words: HashSet<String> = search_words
		.difference(&target_words)
		.cloned()
		.map(|w| w.to_uppercase())
		.collect();

	// Calculate statistics
	let total_words = search_words.len();
	let matched_words = total_words - unmatched_words.len();
	let match_percentage = if total_words > 0 {
		(matched_words as f64 / total_words as f64) * 100.0
	} else {
		0.0
	};

	// Replace unmatched words with highlighted versions using regex
	let highlighted_text = word_boundary
		.replace_all(&search_text, |caps: &regex::Captures| {
			let word = caps[0].to_lowercase();
			if target_words.contains(&word) {
				caps[0].to_string()
			} else {
				format!("__", &caps[0], "__")
			}
		})
		.into_owned();

	Ok(AnalysisResult {
		total_words,
		matched_words,
		match_percentage,
		highlighted_text,
		// unmatched_words,
	})
}

#[derive(Debug, Modal)]
#[name = "Words In The Bible"]
struct BibleModal {
	#[placeholder = "Whether or not to show the message."]
	ephemeral: Option<String>,
}

/// Checks how many words are in the Bible.
#[poise::command(
	context_menu_command = "Words In The Bible",
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn bible(
	ctx: ApplicationContext<'_>,
	message: Message,
) -> Result<()> {
	let data = BibleModal::execute(ctx)
		.await?
		.ok_or_else(|| anyhow!("No modal data."))?;

	let ephemeral = match data.ephemeral.as_deref() {
		Some("false") => false,
		Some(_) => true,
		None => false,
	};

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	let analysis = analyze_text(&message.content, KJV_BIBLE)?;

	let header_text = match analysis.match_percentage {
		100.0 => "This message is certified by The Holy Spirit.",
		0.0 => {
			"ויהי הדבר אשר כתבת, תועבה היא לפני ה׳
כי אין בו אף מילה אחת מדברי הקודש
ומכל דברי התורה והנביאים והכתובים

ארור המסר הזה וארור כותבו
כי לא מצאתי בו אף לא מילה אחת
מספר הספרים אשר נתן לנו האל

על כן כה אמר ה׳:
״הנה ימים באים והשמדתי את המסר הזה
ומחיתי אותו מעל פני האדמה
כי לא היה בו זכר לדברי הקודש״"
		}
		_ => "This message contains traces of Satan's gospel.",
	};

	let text = format!(
		header_text,
		"\n- Message is ",
		analysis.match_percentage:.2,
		"% Holy.\n- ",
		analysis.matched_words,
		"/",
		analysis.total_words,
		" words are in the Bible.\n",
		"### Hightlighted Sins\n",
		analysis.highlighted_text,
	);

	ctx.send(reply.content(text)).await?;

	Ok(())
}
