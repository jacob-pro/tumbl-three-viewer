use serde::Serialize;

pub const UNKNOWN_FILE: &str = "unknown";

#[derive(Serialize, Debug)]
pub struct Post {
    #[serde(flatten)]
    pub common: PostCommon,
    #[serde(flatten)]
    pub r#type: PostType,
}

#[derive(Serialize, Debug)]
pub struct PostCommon {
    pub id: u64,
    pub post_url: Option<String>,
    pub date: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum PostType {
    Image(Image),
    Video(Video),
    Text(Text),
    Answer(Answer),
}

#[derive(Serialize, Debug)]
pub struct Image {
    pub photo_urls: Vec<String>,
    pub caption: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Video {
    pub url: Option<String>,
    pub caption: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Text {
    pub title: Option<String>,
    pub body: String,
    pub media_urls: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct Answer {
    pub body: Option<String>,
}
