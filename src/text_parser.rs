use crate::model::{Answer, Image, Post, PostCommon, PostType, Text, Video};
use crate::utils::create_file_url;
use crate::MetadataType;
use anyhow::Context;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::path::Path;

type TextMap = HashMap<&'static str, String>;

struct Field {
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

const IMAGE_FIELDS: &[Field] = &[
    FIELD_POST_ID,
    FIELD_DATE,
    FIELD_PHOTO_URL,
    FIELD_PHOTO_SET_URLS,
    FIELD_PHOTO_CAPTION,
    FIELD_TAGS,
];

const VIDEO_FIELDS: &[Field] = &[
    FIELD_POST_ID,
    FIELD_DATE,
    FIELD_VIDEO_CAPTION,
    FIELD_VIDEO_PLAYER,
    FIELD_TAGS,
];

const TEXT_FIELDS: &[Field] = &[
    FIELD_POST_ID,
    FIELD_DATE,
    FIELD_TITLE,
    FIELD_BODY,
    FIELD_TAGS,
];

const ANSWER_FIELDS: &[Field] = &[
    FIELD_POST_ID,
    FIELD_DATE,
    FIELD_REBLOG_NAME,
    FIELD_BODY,
    FIELD_TAGS,
];

impl MetadataType {
    /// Parse a text format post
    pub fn parse_text(self, text: String, blog_dir: &Path) -> anyhow::Result<Post> {
        let text_fields = match self {
            MetadataType::Videos => VIDEO_FIELDS,
            MetadataType::Images => IMAGE_FIELDS,
            MetadataType::Texts => TEXT_FIELDS,
            MetadataType::Answers => ANSWER_FIELDS,
        };
        let mut map = read_text_into_map(text, text_fields);
        let common = PostCommon::from_text_map(&mut map)?;
        let specific = match self {
            MetadataType::Videos => PostType::Video(Video::from_text_map(&mut map, blog_dir)),
            MetadataType::Images => PostType::Image(Image::from_text_map(&mut map, blog_dir)),
            MetadataType::Texts => PostType::Text(Text::from_text_map(&mut map)),
            MetadataType::Answers => PostType::Answer(Answer::from_text_map(&mut map)),
        };
        Ok(Post {
            common,
            r#type: specific,
        })
    }
}

fn read_text_into_map(input: String, fields: &[Field]) -> TextMap {
    let mut lines = input.lines().peekable();
    let mut map = HashMap::new();
    for field in fields {
        let prefix = if field.field_name.is_empty() {
            "".to_string()
        } else {
            format!("{}: ", field.field_name)
        };
        let mut cloned = lines.clone();
        while let Some(line) = cloned.next() {
            if prefix.is_empty() || line.starts_with(&prefix) {
                let mut parts = vec![line[prefix.len()..].to_string()];
                while cloned.peek().is_some() && (field.read_next_line)(cloned.peek().unwrap()) {
                    parts.push(cloned.next().unwrap().to_owned());
                }
                map.insert(field.field_name, parts.join("\n"));
                lines = cloned;
                break;
            }
        }
    }
    map
}

/// Split a text file into separate post strings
pub fn split_text_posts(input: String) -> Vec<String> {
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"Post id: \d+$").unwrap());
    let lines = input.lines();
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
    fn from_text_map(map: &mut TextMap) -> anyhow::Result<Self> {
        Ok(PostCommon {
            id: map
                .remove(FIELD_POST_ID.field_name)
                .context("missing id")?
                .parse()?,
            date: map.remove(FIELD_DATE.field_name),
            tags: map
                .remove(FIELD_TAGS.field_name)
                .unwrap_or_default()
                .split(", ")
                .filter(|t| !t.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        })
    }
}

impl Image {
    fn from_text_map(map: &mut TextMap, blog_dir: &Path) -> Self {
        let mut urls = map
            .remove(FIELD_PHOTO_SET_URLS.field_name)
            .unwrap_or_default()
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if urls.is_empty() {
            if let Some(url) = map.remove(FIELD_PHOTO_URL.field_name) {
                urls.push(url);
            }
        }
        if urls.is_empty() {
            log::warn!("Unable to find any photo URLs");
        }
        Self {
            photo_urls: urls
                .iter()
                .map(|u| create_file_url(blog_dir, filename_from_url(u)))
                .collect(),
            caption: map.remove(FIELD_PHOTO_CAPTION.field_name),
        }
    }
}

impl Video {
    fn from_text_map(map: &mut TextMap, blog_dir: &Path) -> Self {
        let url = Self::get_video_url(map, blog_dir)
            .map_err(|e| {
                log::warn!("Unable to get video url: {}", e);
                e
            })
            .ok();
        Video {
            url,
            caption: map.remove(FIELD_VIDEO_CAPTION.field_name),
        }
    }

    fn get_video_url(map: &mut TextMap, blog_dir: &Path) -> anyhow::Result<String> {
        let player = map
            .remove(FIELD_VIDEO_PLAYER.field_name)
            .context("Missing video player")?;
        let fragment = Html::parse_fragment(&player);
        static SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("source").unwrap());
        let video = fragment
            .select(&SELECTOR)
            .next()
            .context("Missing source tag")?;
        let src = video.value().attr("src").context("missing src")?;

        static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"/(tumblr_[a-zA-Z\d]+)").unwrap());
        let captures = REGEX.captures(src).context("Couldn't match video regex")?;
        let filename = format!("{}.mp4", captures.get(1).unwrap().as_str());

        Ok(create_file_url(blog_dir, filename.as_str()))
    }
}

impl Text {
    fn from_text_map(map: &mut TextMap) -> Self {
        Self {
            title: map.remove(FIELD_TITLE.field_name),
            body: map.remove(FIELD_BODY.field_name),
            media_urls: vec![],
        }
    }
}

impl Answer {
    fn from_text_map(map: &mut TextMap) -> Self {
        Self {
            body: map.remove(FIELD_BODY.field_name),
        }
    }
}

fn filename_from_url(url: &str) -> &str {
    if let Some(last_slash) = url.rfind('/') {
        &url[last_slash + 1..]
    } else {
        url
    }
}
