mod model;
mod text_parser;

use crate::model::{Answer, Image, Post, PostCommon, PostType, Text, Video};
use crate::text_parser::{
    read_text_into_map, split_into_posts, Field, TextMap, ANSWER_FIELDS, IMAGE_FIELDS, TEXT_FIELDS,
    VIDEO_FIELDS,
};
use actix_cors::Cors;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::bail;
use clap::Parser;
use env_logger::Env;
use rust_embed::RustEmbed;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tokio::select;

const VIDEOS_FILENAME: &str = "videos.txt";
const IMAGES_FILENAME: &str = "images.txt";
const TEXTS_FILENAME: &str = "texts.txt";
const ANSWERS_FILENAME: &str = "answers.txt";

#[derive(RustEmbed)]
#[folder = "script/"]
struct Viewer;

#[derive(RustEmbed)]
#[folder = "index/"]
struct Index;

async fn blogs(args: Data<Args>) -> HttpResponse {
    let blogs = web::block(move || -> Result<_, String> {
        Ok(fs::read_dir(&args.path)
            .map_err(|_| String::from("Unable to read blog directory"))?
            .filter_map(Result::ok)
            .filter_map(|dir| {
                if !dir.path().is_dir() {
                    return None;
                }
                let name = match dir.path().file_name() {
                    None => return None,
                    Some(name) => name.to_string_lossy().into_owned(),
                };
                if name == "Index" {
                    return None;
                }
                if ![
                    VIDEOS_FILENAME,
                    IMAGES_FILENAME,
                    TEXTS_FILENAME,
                    ANSWERS_FILENAME,
                ]
                .iter()
                .any(|file| dir.path().join(file).exists())
                {
                    return None;
                }
                Some(name)
            })
            .collect::<Vec<_>>())
    })
    .await
    .unwrap();
    match blogs {
        Err(e) => HttpResponse::InternalServerError().body(e),
        Ok(blogs) => HttpResponse::Ok().json(blogs),
    }
}

async fn blog(args: Data<Args>, blog_name: web::Path<String>) -> HttpResponse {
    let res = web::block(move || -> anyhow::Result<_> {
        let dir = args
            .path
            .canonicalize()
            .expect("unable to canonicalize")
            .join(blog_name.into_inner());
        if !dir.is_dir() {
            bail!("Blog directory not found")
        }
        let mut posts = Vec::new();
        posts.extend(load_posts(&dir, IMAGES_FILENAME, IMAGE_FIELDS, |map| {
            PostType::Image(Image::from_text_map(map, &dir))
        })?);
        posts.extend(load_posts(&dir, VIDEOS_FILENAME, VIDEO_FIELDS, |map| {
            PostType::Video(Video::from_text_map(map, &dir))
        })?);
        posts.extend(load_posts(&dir, TEXTS_FILENAME, TEXT_FIELDS, |map| {
            PostType::Text(Text::from_text_map(map))
        })?);
        posts.extend(load_posts(&dir, ANSWERS_FILENAME, ANSWER_FIELDS, |map| {
            PostType::Answer(Answer::from_text_map(map))
        })?);
        posts.sort_by_key(|p| p.common.id);
        Ok(posts)
    })
    .await
    .unwrap();
    match res {
        Err(e) => HttpResponse::BadRequest().body(format!("{}", e)),
        Ok(res) => HttpResponse::Ok().json(res),
    }
}

fn load_posts<M>(
    dir: &Path,
    filename: &str,
    fields: &[Field],
    text_mapper: M,
) -> anyhow::Result<Vec<Post>>
where
    M: Fn(&mut TextMap) -> PostType,
{
    let path = dir.join(filename);
    let mut output = Vec::new();
    if path.is_file() {
        let text = fs::read_to_string(path)?;
        let posts = split_into_posts(text);
        for p in posts {
            let mut map = read_text_into_map(p, fields);
            let common = PostCommon::from_text_map(&mut map)?;
            let specific = text_mapper(&mut map);
            output.push(Post {
                common,
                r#type: specific,
            })
        }
    }
    Ok(output)
}

async fn viewer(path: web::Path<String>) -> HttpResponse {
    let mime = path.rfind('.').map(|idx| {
        let ext = &path[idx + 1..];
        actix_files::file_extension_to_mime(ext)
    });
    match Viewer::get(path.as_str()) {
        Some(file) => HttpResponse::build(StatusCode::OK)
            .content_type(mime.unwrap_or(mime::APPLICATION_OCTET_STREAM))
            .body(file.data.into_owned()),
        None => HttpResponse::NotFound().finish(),
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port number to run web server on
    #[arg(long, default_value_t = 7100)]
    port: u16,
    /// Your TumblThree blogs directory
    #[arg(long, default_value = ".")]
    path: PathBuf,
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let args: Args = Args::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args2 = args.clone();

    let html = Index::get("index.html").unwrap();
    let html = std::str::from_utf8(html.data.as_ref()).unwrap();
    let html = html.replace("${PORT}", &args.port.to_string());
    let mut tempfile = tempfile::Builder::new()
        .suffix(".html")
        .tempfile()
        .expect("Unable to create tempfile");
    tempfile
        .write_all(html.as_bytes())
        .expect("Unable to write index");

    let server = HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .app_data(Data::new(args2.clone()))
            .wrap(cors)
            .route("/blogs", web::get().to(blogs))
            .route("/blogs/{name}", web::get().to(blog))
            .route("/{path:.*}", web::get().to(viewer))
    })
    .bind(("127.0.0.1", args.port))?
    .run();

    open::that(tempfile.path()).expect("Unable to open html");

    select! {
        _ = server => {
            panic!("Server stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("Ctrl-c exiting");
        }
    }
    Ok(())
}
