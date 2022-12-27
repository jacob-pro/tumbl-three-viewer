use crate::model::{Answer, Image, PostCommon, Text, Video};
use anyhow::Context;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::path::Path;

pub type TextMap = HashMap<&'static str, String>;

pub struct Field {
    field_name: &'static str,
    read_next_line: fn(&str) -> bool,
}

impl Field {
    const fn new(field_name: &'static str, read_next_line: fn(&str) -> bool) -> Self {
        Field {
            field_name,
            read_next_line,
        }
    }
}

fn read_one(_: &str) -> bool {
    false
}

fn read_url(next_line: &str) -> bool {
    next_line.starts_with("https://")
}

fn read_until_tags(next_line: &str) -> bool {
    !next_line.starts_with("Tags: ")
}

const FIELD_POST_ID: Field = Field::new("Post id", read_one);
const FIELD_DATE: Field = Field::new("Date", read_one);
const FIELD_TAGS: Field = Field::new("Tags", read_one);
const FIELD_REBLOG_NAME: Field = Field::new("Reblog name", read_one);
const FIELD_BODY: Field = Field::new("", read_until_tags);

const FIELD_PHOTO_URL: Field = Field::new("Photo url", read_one);
const FIELD_PHOTO_SET_URLS: Field = Field::new("Photo set urls", read_url);
const FIELD_PHOTO_CAPTION: Field = Field::new("Photo caption", read_until_tags);

const FIELD_VIDEO_CAPTION: Field = Field::new("Video caption", read_one);
const FIELD_VIDEO_PLAYER: Field = Field::new("Video player", read_until_tags);

const FIELD_TITLE: Field = Field::new("Title", read_one);

pub const IMAGE_FIELDS: &[Field] = &[
    FIELD_POST_ID,
    FIELD_DATE,
    FIELD_PHOTO_URL,
    FIELD_PHOTO_SET_URLS,
    FIELD_PHOTO_CAPTION,
    FIELD_TAGS,
];

pub const VIDEO_FIELDS: &[Field] = &[
    FIELD_POST_ID,
    FIELD_DATE,
    FIELD_VIDEO_CAPTION,
    FIELD_VIDEO_PLAYER,
    FIELD_TAGS,
];

pub const TEXT_FIELDS: &[Field] = &[
    FIELD_POST_ID,
    FIELD_DATE,
    FIELD_TITLE,
    FIELD_BODY,
    FIELD_TAGS,
];

pub const ANSWER_FIELDS: &[Field] = &[
    FIELD_POST_ID,
    FIELD_DATE,
    FIELD_REBLOG_NAME,
    FIELD_BODY,
    FIELD_TAGS,
];

pub fn read_text_into_map(input: String, fields: &[Field]) -> TextMap {
    let mut lines = input.lines().peekable();
    let mut map = HashMap::new();
    for field in fields {
        let prefix = if field.field_name.is_empty() {
            "".to_string()
        } else {
            format!("{}: ", field.field_name)
        };
        while let Some(line) = lines.next() {
            if prefix.is_empty() || line.starts_with(&prefix) {
                let mut parts = vec![line[prefix.len()..].to_string()];
                while lines.peek().is_some() && (field.read_next_line)(lines.peek().unwrap()) {
                    parts.push(lines.next().unwrap().to_owned());
                }
                map.insert(field.field_name, parts.join("\n"));
                break;
            }
        }
    }
    map
}

pub fn split_into_posts(input: String) -> Vec<String> {
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"Post id: \d+$").unwrap());
    let lines = input.lines().peekable();
    let mut output = Vec::new();
    let mut current = Vec::new();
    for line in lines {
        if REGEX.is_match(line) && !current.is_empty() {
            output.push(current.join("\n"));
            current = Vec::new();
        }
        current.push(line);
    }
    if !current.is_empty() {
        output.push(current.join("\n"));
    }
    output
}

impl PostCommon {
    pub fn from_text_map(map: &mut TextMap) -> anyhow::Result<Self> {
        Ok(PostCommon {
            id: map
                .remove(FIELD_POST_ID.field_name)
                .context("missing id")?
                .parse()?,
            date: map.remove(FIELD_DATE.field_name).context("missing id")?,
            tags: map
                .remove(FIELD_TAGS.field_name)
                .context("missing tags")?
                .split(", ")
                .filter(|t| !t.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        })
    }
}

impl Image {
    pub fn from_text_map(map: &mut TextMap, blog_dir: &Path) -> anyhow::Result<Self> {
        let mut urls = map
            .remove(FIELD_PHOTO_SET_URLS.field_name)
            .context("Missing photo set urls")?
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if urls.is_empty() {
            urls = vec![map
                .remove(FIELD_PHOTO_URL.field_name)
                .context("Missing photo url")?];
        }
        Ok(Image {
            photo_urls: urls
                .iter()
                .map(|u| prepend_blog_directory(url_to_file_name(u), blog_dir))
                .collect(),
            caption: map
                .remove(FIELD_PHOTO_CAPTION.field_name)
                .context("Missing photo caption")?,
        })
    }
}

impl Video {
    pub fn from_text_map(map: &mut TextMap, blog_dir: &Path) -> anyhow::Result<Self> {
        let player = map
            .remove(FIELD_VIDEO_PLAYER.field_name)
            .context("Missing video player")?;
        let fragment = Html::parse_fragment(&player);
        let selector = Selector::parse("source").unwrap();
        let video = fragment
            .select(&selector)
            .next()
            .context("Missing source tag")?;
        let src = video.value().attr("src").context("missing src")?;

        static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"/(tumblr_[a-zA-Z\d]+)").unwrap());
        let captures = REGEX.captures(src).context("Couldn't match video regex")?;
        let filename = format!("{}.mp4", captures.get(1).unwrap().as_str());

        let url = prepend_blog_directory(filename.as_str(), blog_dir);

        Ok(Video {
            url,
            caption: map
                .remove(FIELD_VIDEO_CAPTION.field_name)
                .context("Missing video caption")?,
        })
    }
}

impl Text {
    pub fn from_text_map(map: &mut TextMap) -> anyhow::Result<Self> {
        Ok(Text {
            title: map
                .remove(FIELD_TITLE.field_name)
                .context("Missing title")?,
            body: map.remove(FIELD_BODY.field_name).context("Missing body")?,
        })
    }
}

impl Answer {
    pub fn from_text_map(map: &mut TextMap) -> anyhow::Result<Self> {
        Ok(Answer {
            body: map.remove(FIELD_BODY.field_name).context("Missing body")?,
        })
    }
}

fn url_to_file_name(url: &str) -> &str {
    if let Some(last_slash) = url.rfind('/') {
        &url[last_slash + 1..]
    } else {
        url
    }
}

fn prepend_blog_directory(filename: &str, blog_dir: &Path) -> String {
    let path = blog_dir.join(filename).to_string_lossy().to_string();
    let path = path.replace(r"\\?\UNC\", "//");
    let path = path.replace(r"\\?\", "");
    format!("file:///{}", path)
}
