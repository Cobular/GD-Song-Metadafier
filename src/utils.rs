use anyhow::Result;
use glob::glob;
use id3::{Error as id3Error, ErrorKind, Tag, TagLike};
use log::{debug, error};
use std::{
    io,
    path::{Path, PathBuf},
};

/// Gets all song IDs found in the containing folder
pub fn get_all_ids(base_path: String) -> Result<Vec<String>> {
    // Store the file IDs
    let mut file_ids: Vec<String> = Vec::new();

    // A path to use in glob
    let path = format!("{}[0-9]*.mp3", base_path);

    // Generate the list of file IDs
    for entry in glob(&path)? {
        match entry {
            Ok(path) => file_ids.push(
                path.file_stem()
                    .ok_or(make_io_err("File has no stem"))?
                    .to_str()
                    .ok_or(make_io_err("Failed to parse stem to str"))?
                    .to_string(),
            ),
            Err(e) => log::error!("{:?}", e),
        }
    }
    Ok(file_ids)
}

/// Get IDs of songs that don't have titles
pub fn get_non_title_ids(base_path: String) -> Result<Vec<String>> {
    // Store the file IDs
    let mut file_ids: Vec<String> = Vec::new();

    // A path to use in glob
    let path = format!("{}[0-9]*.mp3", base_path);

    // Generate the list of file IDs
    for entry in glob(&path)? {
        match entry {
            Ok(path) => {
                // Only do work if file does not have title
                debug!("Checking {}...", path.display());
                let tag = match Tag::read_from_path(path.as_path()) {
                    Ok(tag) => {
                        debug!("Found a tag on {}", path.display());
                        tag
                    }
                    Err(id3Error {
                        kind: ErrorKind::NoTag,
                        ..
                    }) => {
                        debug!("No tag on {}", path.display());
                        Tag::new()
                    }
                    Err(id3Error {
                        kind: ErrorKind::Parsing,
                        ..
                    }) => {
                        debug!("Failed to parse the tag on {}, giving it a new tag", path.display());
                        Tag::new()
                    }
                    Err(err) => {
                        error!(
                            "Error reading tag on {:?}: {:?}",
                            path, err
                        );
                        continue;
                    }
                };
                if tag.title().is_none() {
                    debug!("{} has no title", path.display());
                    file_ids.push(
                        path.file_stem()
                            .ok_or(make_io_err("File has no stem"))?
                            .to_str()
                            .ok_or(make_io_err("Failed to parse stem to str"))?
                            .to_string(),
                    )
                }
            }
            Err(e) => error!("{:?}", e),
        }
    }
    Ok(file_ids)
}

pub fn make_io_err(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, text)
}

pub fn make_path_from_id(base_path: &str, id: &str) -> PathBuf {
    let mut file_path = Path::new(&base_path).to_path_buf();
    file_path.set_file_name(&id);
    file_path.set_extension("mp3");
    file_path
}
