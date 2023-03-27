use std::path::Path;

pub fn create_file_url(blog_dir: &Path, filename: &str) -> String {
    let path = blog_dir.join(filename).to_string_lossy().to_string();
    let path = path.replace(r"\\?\UNC\", "//");
    let path = path.replace(r"\\?\", "");
    let path = path.replace('\\', "/");
    format!("file:///{}", path)
}
