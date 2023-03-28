use std::path::{Path, PathBuf};

pub fn create_file_url(blog_dir: &Path, filename: &str) -> String {
    let path = blog_dir.join(filename).to_string_lossy().to_string();
    let path = path.replace(r"\\?\UNC\", "//");
    let path = path.replace(r"\\?\", "");
    let path = path.replace('\\', "/");
    format!("file:///{}", path)
}

pub struct BlogDir {
    pub path: PathBuf,
    pub files: Vec<String>,
}

impl BlogDir {
    pub fn new(path: &Path) -> Self {
        let list = std::fs::read_dir(path).expect("Unable to read blog directory");
        let files = list
            .into_iter()
            .flatten()
            .filter(|r| r.file_type().unwrap().is_file())
            .map(|r| r.file_name().to_string_lossy().to_string())
            .collect();
        Self {
            path: path.to_path_buf(),
            files,
        }
    }

    pub fn find_file_starting_with(&self, starting_with: &str) -> Option<String> {
        self.files
            .iter()
            .find(|f| f.starts_with(starting_with))
            .cloned()
    }
}
