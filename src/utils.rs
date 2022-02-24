use glob::glob;
use id3::{Error as id3Error, ErrorKind, Tag, TagLike};
use log::error;
use std::{io, path::{PathBuf, Path}};
use anyhow::Result;

/// Gets all song IDs found in the containing folder
pub fn get_all_ids(base_path: String) -> Result<Vec<String>> {
    // Store the file IDs
    let mut file_ids: Vec<String> = Vec::new();

    // A path to use in glob
    let path = format!("{}[0-9]*.mp3", base_path);

    // Generate the list of file IDs
    for entry in glob(&path)? {
        match entry {
            Ok(path) => {
                file_ids.push(
                    path.file_stem()
                        .ok_or(make_io_err("File has no stem"))?
                        .to_str()
                        .ok_or(make_io_err("Failed to parse stem to str"))?
                        .to_string(),
                )
            }
            Err(e) => log::error!("{:?}", e),
        }
    }
    Ok(file_ids)
}

///
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
                println!("{}", path.display());
                let tag = match Tag::read_from_path(path.as_path()) {
                    Ok(tag) => tag,
                    Err(id3Error {
                        kind: ErrorKind::NoTag,
                        ..
                    }) => Tag::new(),
                    Err(id3Error {
                        kind: ErrorKind::Parsing,
                        ..
                    }) => Tag::new(),
                    Err(err) => {
                        error!("Error getting metadata status of file {:?}: {:?}", path, err);
                        continue;
                    }
                };
                if tag.title().is_none() {
                    file_ids.push(
                        path.file_stem()
                            .ok_or(make_io_err("File has no stem"))?
                            .to_str()
                            .ok_or(make_io_err("Failed to parse stem to str"))?
                            .to_string(),
                    )
                }
            }
            Err(e) => println!("{:?}", e),
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