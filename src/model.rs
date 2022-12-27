use serde::Serialize;

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
    pub date: String,
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
    pub caption: String,
}

#[derive(Serialize, Debug)]
pub struct Video {
    pub url: String,
    pub caption: String,
}

#[derive(Serialize, Debug)]
pub struct Text {
    pub title: String,
    pub body: String,
}

#[derive(Serialize, Debug)]
pub struct Answer {
    pub body: String,
}
