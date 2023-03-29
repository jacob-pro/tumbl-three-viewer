use crate::model::{Answer, Image, Post, PostCommon, PostType, Text, Video, UNKNOWN_FILE};
use crate::utils::{create_file_url, BlogDir};
use crate::MetadataType;
use itertools::Itertools;
use lol_html::{element, RewriteStrSettings};
use serde::Deserialize;

#[derive(Deserialize)]
struct JsonCommon {
    id: String,
    date: String,
    tags: Vec<String>,
    #[serde(default)]
    #[serde(alias = "downloaded-media-files")]
    downloaded_media_files: Vec<String>,
}

impl JsonCommon {
    fn to_model(&self) -> anyhow::Result<PostCommon> {
        Ok(PostCommon {
            id: self.id.parse()?,
            date: Some(self.date.clone()),
            tags: self.tags.clone(),
        })
    }
}

#[derive(Deserialize)]
pub struct JsonVideo {
    #[serde(flatten)]
    common: JsonCommon,
    caption: String,
}

impl JsonVideo {
    fn into_post(self, blog_dir: &BlogDir) -> anyhow::Result<Post> {
        if self.common.downloaded_media_files.len() != 1 {
            log::warn!(
                "Unexpected downloaded_media_files for video {}",
                self.common.id
            );
        }
        Ok(Post {
            common: self.common.to_model()?,
            r#type: PostType::Video(Video {
                url: self
                    .common
                    .downloaded_media_files
                    .first()
                    .map(|filename| url_for_media_file(blog_dir, filename)),
                caption: Some(self.caption),
            }),
        })
    }
}

#[derive(Deserialize)]
struct JsonImage {
    #[serde(flatten)]
    common: JsonCommon,
    #[serde(alias = "photo-caption")]
    caption: String,
}

impl JsonImage {
    fn into_post(self, blog_dir: &BlogDir) -> anyhow::Result<Post> {
        if self.common.downloaded_media_files.is_empty() {
            log::warn!(
                "Missing downloaded_media_files for image {}",
                self.common.id
            );
        }
        Ok(Post {
            common: self.common.to_model()?,
            r#type: PostType::Image(Image {
                photo_urls: self
                    .common
                    .downloaded_media_files
                    .iter()
                    .map(|filename| url_for_media_file(blog_dir, filename))
                    .collect(),
                caption: Some(self.caption),
            }),
        })
    }
}

#[derive(Deserialize)]
struct JsonText {
    #[serde(flatten)]
    common: JsonCommon,
    #[serde(default)]
    #[serde(alias = "regular-body")]
    body: String,
    #[serde(alias = "regular-title")]
    title: Option<String>,
}

impl JsonText {
    fn into_post(self, blog_dir: &BlogDir) -> anyhow::Result<Post> {
        let common = self.common.to_model()?;
        // A text post may have images and videos within the body
        // We must rewrite the body HTML to remove the remote URLs
        let element_content_handlers = vec![
            element!("img", |el| {
                el.remove();
                Ok(())
            }),
            element!("figure", |el| {
                el.remove();
                Ok(())
            }),
            element!("video", |el| {
                el.remove();
                Ok(())
            }),
        ];
        let body = lol_html::rewrite_str(
            &self.body,
            RewriteStrSettings {
                element_content_handlers,
                ..RewriteStrSettings::default()
            },
        )
        .unwrap();
        let media = self
            .common
            .downloaded_media_files
            .into_iter()
            .unique()
            .map(|filename| url_for_media_file(blog_dir, &filename))
            .collect();

        Ok(Post {
            common,
            r#type: PostType::Text(Text {
                title: self.title,
                body,
                media_urls: media,
            }),
        })
    }
}

#[derive(Deserialize)]
struct JsonAnswer {
    #[serde(flatten)]
    common: JsonCommon,
    question: String,
    answer: String,
}

impl JsonAnswer {
    fn into_post(self) -> anyhow::Result<Post> {
        let answer = format!("<em>{}</em><br>{}", self.question, self.answer);
        Ok(Post {
            common: self.common.to_model()?,
            r#type: PostType::Answer(Answer { body: Some(answer) }),
        })
    }
}

impl MetadataType {
    /// Parse a JSON format post
    pub fn parse_json(self, json: serde_json::Value, blog_dir: &BlogDir) -> anyhow::Result<Post> {
        match self {
            MetadataType::Videos => serde_json::from_value::<JsonVideo>(json)?.into_post(blog_dir),
            MetadataType::Images => serde_json::from_value::<JsonImage>(json)?.into_post(blog_dir),
            MetadataType::Texts => serde_json::from_value::<JsonText>(json)?.into_post(blog_dir),
            MetadataType::Answers => serde_json::from_value::<JsonAnswer>(json)?.into_post(),
        }
    }
}

/// Resolve a file URL for an item in the `downloaded_media_files` array
fn url_for_media_file(blog_dir: &BlogDir, downloaded_media_file: &str) -> String {
    let mut search_prefix = downloaded_media_file;
    // Trim the extension to workaround https://github.com/TumblThreeApp/TumblThree/issues/439
    if let Some(dot_index) = downloaded_media_file.rfind('.') {
        search_prefix = &downloaded_media_file[0..dot_index + 1];
    }

    if let Some(matched) = blog_dir.find_file_starting_with(search_prefix) {
        if matched != downloaded_media_file {
            log::warn!("Rewriting file {} to {}", downloaded_media_file, matched);
        }
        create_file_url(&blog_dir.path, &matched)
    } else {
        log::warn!("Unable to find file matching {}", downloaded_media_file);
        String::from(UNKNOWN_FILE)
    }
}
