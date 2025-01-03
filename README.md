# discord_selfbot

A personal-use Discord app created to replicate the useful functionality of ye selfbots of olde with official bot APIs.

## Installation

### Requirements

- Git
- Docker

### Setup

```sh
git clone https://github.com/Kyza/discord_selfbot.git
cd discord_selfbot
```

Create a copy of the `example.config.ron` file and rename it to `config.ron`. Then edit it to your situation.

You'll need to create a bot on the [Discord developer portal](https://discord.com/developers/applications) and get the token and application ID for it.

Make sure you enable the `User Install` installation context. It's also recommended that you disable `Public Bot` as well.

Use the Discord-provided installation link to install the bot on your account.

```sh
docker compose up
```

This might take a while the first time you run it.

## Notes

By default the bot should send messages as non-ephemeral, but if you want to make it ephemeral you can use the `ephemeral` argument.

As long as you don't change any files you can use `update_restart.cmd` to pull the latest code and restart it.

## Commands

- [x] `/age` - Sends the timestamp of the ID or user's creation date.
- [x] `/bible` - Checks how many words are in the Bible.
   - [x] Context menu supported.
- [ ] `/cobalt` - Downloads media from a URL using the Cobalt API and sends it.
   - Disabled until Cobalt gets an official API.
- [x] `/help` - Shows the help menu.
- [x] `/embed` - Creates and sends an embed from either fields or multiple from a RON representation.
   - https://github.com/ron-rs/ron
   - https://docs.rs/poise/latest/poise/serenity_prelude/struct.Embed.html
- [x] `/escape` - Escapes basic markdown characters.
- [x] `/favoritize` - Converts any image type into a 2 frame WebP so that it can be added to your favorited GIFs list.
   - [x] Context menu supported.
- [x] `/ffmpeg` - Runs a basic FFmpeg command on uploaded media.
- [x] `/fix` - Makes social media links embed properly.
   - Works for X, Bluesky, TikTok, Instagram, and Reddit.
- [x] `/github` - Sends a formatted link to a GitHub profile or repository.
- [x] `/jxl` - Converts an image to JXL.
   - [x] Context menu supported.
- [x] `/ocr` - Runs OCR on an image.
   - This sucks currently. Someone please find me a decent API or library.
- [x] `/roll` - Rolls dice notation.
   - Uses a [custom unlimited version of the `caith` crate](https://github.com/Kyza/caith/commit/a05c6a3954ab3f42d4ce08d8de18fe5a2fae18b6).
- [x] `/screenshot` - Screenshots a website.
- [x] `/snowstamp` - Lets you easily create a timestamp from an ID or a datetime.
- [x] `/translate` - Translates text using DeepL.
   - [x] Context menu supported.
   - DeepL's API signup has been broken for me for the past few months, so this uses `thirtyfour` and `geckodriver`.
      - It can only handle one translation at a time.
      - Firefox currently has a bug where it has to be run with `sudo` to work.
      - I have some code that will run a new `geckodriver` instance for each translation, but it's not working yet because of that and more.
      - https://github.com/SeleniumHQ/selenium/issues/12862
      - https://github.com/mozilla/geckodriver/issues/2082
- [x] `/unicode` - Converts text to and from Unicode.
- [x] `/uptime` - Tells you how long the bot has been up for.
- [x] `/wayback` - Generates an archive.org (Wayback Machine) URL for a given URL.
- [x] `/webp` - Converts an image to WebP.
   - [x] Context menu supported.
- [x] `/wolfram` - Asks Wolfram Alpha a question.
- [ ] `/youtube` - [Experimental] Downloads and sends a YouTube video and sends it.
   - Sometimes it works, sometimes it doesn't.
