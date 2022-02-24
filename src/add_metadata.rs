use std::{io, path::PathBuf, str::FromStr};

use anyhow::Result;
use futures::{stream, StreamExt};
use id3::{Content, Error as id3Error, ErrorKind, Frame, Tag, TagLike, Timestamp, Version};
use log::{debug, error, info, warn};
use youtube_dl::{Error, SingleVideo, YoutubeDl, YoutubeDlOutput};

use crate::utils::{get_non_title_ids, make_io_err, make_path_from_id};

pub async fn add_metadata(base_path: PathBuf, parallel_requests: usize) -> Result<()> {
    let base_path = base_path.to_str().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "Failed to parse FilePath",
    ))?;

    let file_ids = get_non_title_ids(base_path.to_string())?;

    debug!("Found file IDs: {:?}", file_ids);

    let song_bodies = stream::iter(file_ids)
        .map(|file_id| {
            let url = format!("https://www.newgrounds.com/audio/listen/{}", file_id);
            debug!("Querying URL {}", url);
            async move { YoutubeDl::new(url).socket_timeout("15").run_async().await }
        })
        .buffer_unordered(parallel_requests);

    song_bodies
        .for_each(|body| async {
            match body {
                Ok(output) => {
                    parse_successful(output, base_path.to_string()).err();
                }

                Err(err) => match err {
                    Error::Io(err) => panic!("IO error: {:?}", err),
                    Error::Json(err) => panic!("JSON error: {:?}", err),
                    Error::ExitCode { code, stderr } => {
                        if stderr.contains("HTTP Error 404") {
                            error!(
                                "One of your files seems to have 404ed! Check that it exists. \n{:?}", stderr
                            );
                        } else {
                            error!("YoutubeDl Exited with code {} -- {:?}", code, stderr);
                        }
                    }
                    Error::ProcessTimeout => error!("Process timed out"),
                },
            }
        })
        .await;

    Ok(())
}

fn parse_successful(ytdl_output: YoutubeDlOutput, base_path: String) -> Result<()> {
    match ytdl_output {
        YoutubeDlOutput::Playlist(_) => {
            error!("One of your songs seems to have been a playlist! We can't process those.");
            Ok(())
        }
        YoutubeDlOutput::SingleVideo(vid) => {
            let SingleVideo {
                title,
                webpage_url,
                url,
                uploader,
                upload_date,
                display_id,
                ..
            } = *vid;

            debug!("Successfully parsed {}", title);

            // Display ID is critical to the functionality as it's how we get back to the song file
            let display_id = display_id.ok_or(make_io_err(&format!(
                "Song {} does not have a display ID",
                &title
            )))?;
        

            debug!("{:?}", base_path);
            let file_path = make_path_from_id(&base_path, &display_id);

            debug!("Trying to open file at path {:?}", file_path);

            let file_name = file_path.file_name().unwrap_or(file_path.as_os_str());

            let mut tag = match Tag::read_from_path(file_path.as_path()) {
                Ok(tag) => {
                    debug!("Found a tag on {:?}", file_name);
                    tag
                }
                Err(id3Error {
                    kind: ErrorKind::NoTag,
                    ..
                }) => {
                    debug!("No tag on {:?}", file_name);
                    Tag::new()
                }
                Err(id3Error {
                    kind: ErrorKind::Parsing,
                    description,
                    ..
                }) => {
                    warn!("Failed to parse the tag on {:?} ({:?}), giving it a new tag", file_name, description);
                    Tag::new()
                }
                Err(err) => {
                    error!(
                        "Error reading tag on {:?}: {:?}",
                        file_name, err
                    );
                    Err(err)?
                }
            };

            debug!("Current tags: {:?}", tag.title());

            tag.set_title(&title);
            debug!("Set title for {} to {}", display_id, &title);
            let mut count: u8 = 1;

            if let Some(uploader) = uploader {
                tag.set_artist(&uploader);
                debug!("Set artist for {} to {}", display_id, &uploader);
                count += 1;
            }

            if let Some(upload_date) = upload_date {
                tag.set_date_released(Timestamp::from_str(&upload_date).map_err(|err| {
                    error!("Failed to parse timestamp for upload!");
                    err
                })?);
                debug!("Set upload date for {} to {}", display_id, upload_date);
                count += 1;
            }

            // WOAS - Official audio source webpage
            if let Some(webpage_url) = webpage_url {
                let frame = Frame::with_content("WOAS", Content::Link(webpage_url.clone()));
                tag.add_frame(frame);
                debug!(
                    "Set audio source webpage for {} to {}",
                    display_id, &webpage_url
                );
                count += 1;
            }

            // WOAF - Official audio file webpage
            if let Some(url) = url {
                let frame = Frame::with_content("WOAF", Content::Link(url.clone()));
                tag.add_frame(frame);
                debug!("Set audio file webpage for {} to {}", &display_id, &url);
                count += 1;
            }

            // TXXX - User defined text information frame
            let frame = Frame::with_content("TXXX", Content::Text(display_id.to_string()));
            tag.add_frame(frame);
            debug!("Set user defined info for {} to {}", display_id, display_id);
            count += 1;

            info!("Saving data to file {}", file_path.display());

            match tag.write_to_path(&file_path, Version::Id3v24) {
                Ok(_) => debug!("{}/6 tags successfully written to {:?}", count, file_path),
                Err(err) => warn!("Tag failed to write: {:?}", err),
            };

            Ok(())
        }
    }
}
