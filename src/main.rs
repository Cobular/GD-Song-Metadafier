use std::{
    panic,
    path::{Path, PathBuf},
    str::FromStr,
};

use clap::Parser;
use futures::{stream, StreamExt};
use glob::glob;
use id3::{Content, Error as id3Error, ErrorKind, Frame, Tag, TagLike, Timestamp, Version};
use tokio;
use youtube_dl::{Error, SingleVideo, YoutubeDl, YoutubeDlOutput};

/// A tool to find and assign metadata to Geometry Dash music files
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    /// The path to the GD music folder, including trailing slash
    #[clap(long, parse(from_os_str), default_value = "./")]
    path: PathBuf,

    /// The number of concurrent requests to make
    #[clap(long, default_value = "4")]
    parallel_requests: usize,
}

#[tokio::main]
async fn main() {
    // Arg parser - makes it a proper command line app
    let Args {
        path: base_path,
        parallel_requests,
    } = Args::parse();

    let path = format!("{}[0-9]*.mp3", base_path.to_str().unwrap());
    let mut file_ids: Vec<String> = Vec::new();

    // Generate the list of file IDs
    for entry in glob(&path).expect("Glob pattern failed") {
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
                        // Abort on a broken file I guess?
                        eprintln!("Error on file {:?}: {}", path, err);
                        continue;
                    }
                };
                println!("{}", tag.title().unwrap_or(&format!("No title on {}", path.display())));
                if tag.title().is_none() {
                    file_ids.push(
                        path.file_stem()
                            .expect("File does not have stem")
                            .to_str()
                            .expect("File stem to str failed")
                            .to_string(),
                    )
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }
    let bodies = stream::iter(file_ids)
        .map(|file_id| {
            let url = format!("https://www.newgrounds.com/audio/listen/{}", file_id);
            async move { YoutubeDl::new(url).socket_timeout("15").run_async().await }
        })
        .buffer_unordered(parallel_requests);

    bodies
        .for_each(|body| async {
            match body {
                Ok(output) => {
                    match output {
                        YoutubeDlOutput::Playlist(_) => panic!("This does not support playlists"),
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

                            // Display ID is critical to the functionality as it's how we get back to the song file
                            let display_id = display_id.unwrap();
                            let mut file_path = Path::new(&base_path).to_path_buf();
                            file_path.set_file_name(&display_id);
                            file_path.set_extension("mp3");
                            let mut tag = match Tag::read_from_path(&file_path) {
                                Ok(tag) => tag,
                                Err(id3Error {
                                    kind: ErrorKind::NoTag,
                                    ..
                                }) => Tag::new(),
                                Err(id3Error {
                                    kind: ErrorKind::Parsing,
                                    ..
                                }) => Tag::new(),
                                Err(err) => panic!("{}", err),
                            };

                            tag.set_title(title);

                            if let Some(uploader) = uploader {
                                tag.set_artist(uploader);
                            }

                            if let Some(upload_date) = upload_date {
                                tag.set_date_released(
                                    Timestamp::from_str(&upload_date)
                                        .expect("Failed to parse timestamp"),
                                );
                            }

                            // WOAS - Official audio source webpage
                            if let Some(webpage_url) = webpage_url {
                                let frame = Frame::with_content("WOAS", Content::Link(webpage_url));
                                tag.add_frame(frame);
                            }

                            // WOAF - Official audio file webpage
                            if let Some(url) = url {
                                let frame = Frame::with_content("WOAF", Content::Link(url));
                                tag.add_frame(frame);
                            }

                            // TXXX - User defined text information frame
                            let frame = Frame::with_content("TXXX", Content::Text(display_id));
                            tag.add_frame(frame);

                            println!("Saving data to file {}", file_path.display());

                            tag.write_to_path(&file_path, Version::Id3v22).unwrap();
                        }
                    };
                }
                Err(err) => match err {
                    Error::Io(err) => panic!("IO error: {:?}", err),
                    Error::Json(err) => panic!("JSON error: {:?}", err),
                    Error::ExitCode { code, stderr } => {
                        if stderr.contains("HTTP Error 404") {
                            eprintln!(
                                "One of your files seems to have 404ed! Check that it exists."
                            )
                        } else {
                            panic!("YoutubeDl Exited with code {}\n{:?}", code, stderr);
                        }
                    }
                    Error::ProcessTimeout => panic!("Process timed out"),
                },
            }
        })
        .await;
}
