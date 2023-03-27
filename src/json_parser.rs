use crate::model::{Answer, Image, Post, PostCommon, PostType, Text, Video};
use crate::utils::create_file_url;
use crate::MetadataType;
use itertools::Itertools;
use lol_html::{element, RewriteStrSettings};
use serde::Deserialize;
use std::path::Path;

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
    caption: Option<String>,
}

impl JsonVideo {
    fn into_post(self, blog_dir: &Path) -> anyhow::Result<Post> {
        if self.common.downloaded_media_files.len() != 1 {
            log::warn!("unexpected media length for post {}", self.common.id);
        }
        Ok(Post {
            common: self.common.to_model()?,
            r#type: PostType::Video(Video {
                url: self
                    .common
                    .downloaded_media_files
                    .first()
                    .map(|filename| create_file_url(blog_dir, filename)),
                caption: self.caption,
            }),
        })
    }
}

#[derive(Deserialize)]
struct JsonImage {
    #[serde(flatten)]
    common: JsonCommon,
    caption: Option<String>,
}

impl JsonImage {
    fn into_post(self, blog_dir: &Path) -> anyhow::Result<Post> {
        Ok(Post {
            common: self.common.to_model()?,
            r#type: PostType::Image(Image {
                photo_urls: self
                    .common
                    .downloaded_media_files
                    .iter()
                    .map(|filename| create_file_url(blog_dir, filename))
                    .collect(),
                caption: self.caption,
            }),
        })
    }
}

#[derive(Deserialize)]
struct JsonText {
    #[serde(flatten)]
    common: JsonCommon,
    #[serde(default)]
    body: String,
}

impl JsonText {
    fn into_post(self, blog_dir: &Path) -> anyhow::Result<Post> {
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
        let output = lol_html::rewrite_str(
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
            .map(|filename| create_file_url(blog_dir, &filename))
            .collect();

        Ok(Post {
            common,
            r#type: PostType::Text(Text {
                title: None,
                body: Some(output),
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
    pub fn parse_json(self, json: serde_json::Value, blog_dir: &Path) -> anyhow::Result<Post> {
        match self {
            MetadataType::Videos => serde_json::from_value::<JsonVideo>(json)?.into_post(blog_dir),
            MetadataType::Images => serde_json::from_value::<JsonImage>(json)?.into_post(blog_dir),
            MetadataType::Texts => serde_json::from_value::<JsonText>(json)?.into_post(blog_dir),
            MetadataType::Answers => serde_json::from_value::<JsonAnswer>(json)?.into_post(),
        }
    }
}
