use std::path::{Path, PathBuf};

pub fn add_extension<P: AsRef<Path>, Q: AsRef<Path>>(path: P, extension: Q) -> PathBuf {
    match path.as_ref().extension() {
        Some(ext) => {
            let mut ext = ext.to_os_string();
            ext.push(".");
            ext.push(extension.as_ref());
            path.as_ref().with_extension(ext)
        }
        None => path.as_ref().with_extension(extension.as_ref()),
    }
}
