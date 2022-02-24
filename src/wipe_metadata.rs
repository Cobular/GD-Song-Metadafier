use std::{io, path::PathBuf};

use anyhow::Result;
use id3::{Tag, Version};
use log::{debug, error, info};

use crate::utils::{get_all_ids, make_path_from_id};

pub fn wipe_metadata(base_path: PathBuf) -> Result<()> {
    let base_path = base_path.to_str().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "Failed to parse FilePath",
    ))?;
    let all_ids = get_all_ids(base_path.to_string())?;

    info!("Wiping {} files", all_ids.len());

    let tag = Tag::new();

    for id in all_ids {
        let file_path = make_path_from_id(&base_path, &id);

        tag.write_to_path(&file_path, Version::Id3v24).map_err(|err| {
            error!("Failed to write wiped tag: {:#?}", err);
            err
        })?;
        debug!("File {:?} had metadata wiped", file_path);
    }

    Ok(())
}
