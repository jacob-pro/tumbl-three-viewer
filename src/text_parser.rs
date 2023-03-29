use crate::model::{Answer, Image, Post, PostCommon, PostType, Text, Video, UNKNOWN_FILE};
use crate::utils::{create_file_url, BlogDir};
use crate::MetadataType;
use anyhow::{bail, Context};
use lol_html::{element, RewriteStrSettings};
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
    pub fn parse_text(self, text: String, blog_dir: &BlogDir) -> anyhow::Result<Post> {
        let text_fields = match self {
            MetadataType::Videos => VIDEO_FIELDS,
            MetadataType::Images => IMAGE_FIELDS,
            MetadataType::Texts => TEXT_FIELDS,
            MetadataType::Answers => ANSWER_FIELDS,
        };
        let mut map = read_text_into_map(text, text_fields);
        let common = PostCommon::from_text_map(&mut map)?;
        let specific = match self {
            MetadataType::Videos => {
                PostType::Video(Video::from_text_map(&mut map, &blog_dir.path, common.id))
            }
            MetadataType::Images => PostType::Image(Image::from_text_map(&mut map, blog_dir)),
            MetadataType::Texts => PostType::Text(Text::from_text_map(&mut map, blog_dir)),
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
    fn from_text_map(map: &mut TextMap, blog_dir: &BlogDir) -> Self {
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
                .map(|u| rewrite_image_url(u, blog_dir))
                .collect(),
            caption: map.remove(FIELD_PHOTO_CAPTION.field_name),
        }
    }
}

impl Video {
    fn from_text_map(map: &mut TextMap, blog_dir: &Path, post_id: u64) -> Self {
        let url: anyhow::Result<_> = (|| {
            let player = map
                .remove(FIELD_VIDEO_PLAYER.field_name)
                .context("Missing 'Video player' field")?;
            let fragment = Html::parse_fragment(&player);
            static SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("source").unwrap());
            let video = fragment
                .select(&SELECTOR)
                .next()
                .context("Missing 'source' tag")?;
            let src = video
                .value()
                .attr("src")
                .context("Source element missing 'src' attribute")?;

            static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"/(tumblr_[a-zA-Z\d]+)").unwrap());
            let captures = REGEX
                .captures(src)
                .context("Couldn't find a supported video URL")?;
            let filename = format!("{}.mp4", captures.get(1).unwrap().as_str());

            Ok(create_file_url(blog_dir, filename.as_str()))
        })();
        if let Err(e) = &url {
            log::warn!("Unable to find a video URL for post {}: {}", post_id, e);
        }
        Video {
            url: url.ok(),
            caption: map.remove(FIELD_VIDEO_CAPTION.field_name),
        }
    }
}

impl Text {
    fn from_text_map(map: &mut TextMap, blog_dir: &BlogDir) -> Self {
        let body = map.remove(FIELD_BODY.field_name).unwrap_or_default();

        // A text post may have images within the body
        // We must rewrite the body HTML to replace with local URLs
        let element_content_handlers = vec![element!("img[src]", |el| {
            let src = el.get_attribute("src").unwrap();
            let replacement = rewrite_image_url(&src, blog_dir);
            el.set_attribute("src", &replacement)?;
            Ok(())
        })];
        let body = lol_html::rewrite_str(
            &body,
            RewriteStrSettings {
                element_content_handlers,
                ..RewriteStrSettings::default()
            },
        )
        .unwrap();

        Self {
            title: map.remove(FIELD_TITLE.field_name),
            body,
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

/// Rewrite an Tumblr image URL to a file on disk
/// This relies on the assumption that TumblThree wasn't configured to rewrite the file names
/// Used for metadata when the `downloaded_media_files` feature wasn't available
/// https://github.com/TumblThreeApp/TumblThree/commit/62373027d7b5d13d548be90104a5f265a719ed64
fn rewrite_image_url(url: &str, blog_dir: &BlogDir) -> String {
    let result = (|| -> anyhow::Result<_> {
        let slash_idx = url.rfind('/').context("Unable to find '/' in url")? + 1;
        let url_filename = &url[slash_idx..];
        let mut search_prefix = url_filename;

        // Work around for some images where the URL size suffix does not match
        // the one on disk, e.g. _540.jpg vs _1280.jpg
        if search_prefix.starts_with("tumblr_") {
            if let Some(last_underscore_idx) = search_prefix.rfind('_') {
                search_prefix = &search_prefix[..last_underscore_idx + 1];
            }
        }

        if let Some(matched) = blog_dir.find_file_starting_with(search_prefix) {
            if matched != url_filename {
                log::warn!("Rewriting file {} to {}", url_filename, matched);
            }
            return Ok(create_file_url(&blog_dir.path, &matched));
        }
        bail!("No file on disk starting with: {}", search_prefix);
    })();
    match result {
        Ok(url) => url,
        Err(e) => {
            log::warn!("Unable to rewrite URL '{}': {}", url, e);
            String::from(UNKNOWN_FILE)
        }
    }
}
