mod json_parser;
mod model;
mod text_parser;
mod utils;

use crate::model::Post;
use crate::text_parser::split_text_posts;
use actix_cors::Cors;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::bail;
use clap::Parser;
use enum_iterator::Sequence;
use env_logger::Env;
use rust_embed::RustEmbed;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tokio::select;

const VIDEOS_FILENAME: &str = "videos.txt";
const IMAGES_FILENAME: &str = "images.txt";
const TEXTS_FILENAME: &str = "texts.txt";
const ANSWERS_FILENAME: &str = "answers.txt";

#[derive(Copy, Clone, Sequence)]
enum MetadataType {
    Videos,
    Images,
    Texts,
    Answers,
}

impl MetadataType {
    pub fn file_name(self) -> &'static str {
        match self {
            MetadataType::Videos => "videos.txt",
            MetadataType::Images => "images.txt",
            MetadataType::Texts => "texts.txt",
            MetadataType::Answers => "answers.txt",
        }
    }
}

#[derive(RustEmbed)]
#[folder = "script/"]
struct Viewer;

#[derive(RustEmbed)]
#[folder = "index/"]
struct Index;

/// Route handler for /blogs - returns list of all directories that contain one or more
/// metadata files
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

/// Route handler for /blog/{name}
/// Return a list of posts
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
        for file in enum_iterator::all::<MetadataType>() {
            posts.extend(load_posts(&dir, file)?);
        }
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

/// Loads all posts from a metadata file (if it exists)
fn load_posts(dir: &Path, metadata_type: MetadataType) -> anyhow::Result<Vec<Post>> {
    let path = dir.join(metadata_type.file_name());
    if path.is_file() {
        let text = fs::read_to_string(path)?;
        if text.starts_with('[') {
            serde_json::from_str::<Vec<serde_json::Value>>(&text)?
                .into_iter()
                .map(|json| metadata_type.parse_json(json, dir))
                .collect::<Result<Vec<_>, _>>()
        } else {
            let blog_dir = text_parser::BlogDir::new(dir);
            split_text_posts(text)
                .into_iter()
                .map(|text| metadata_type.parse_text(text, &blog_dir))
                .collect::<Result<Vec<_>, _>>()
        }
    } else {
        Ok(vec![])
    }
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
    let tempfile = std::env::temp_dir().join(format!("tumbl-three-viewer-{}.html", args.port));
    fs::write(&tempfile, html).expect("Unable to create tempfile");

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

    log::info!("Opening: {}", tempfile.display());
    open::that(tempfile).expect("Unable to open html");

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
