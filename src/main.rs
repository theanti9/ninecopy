mod args;
mod errors;
mod stats;

use std::{
    collections::VecDeque,
    fs::Metadata,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::Instant,
};

use args::Args;
use byte_unit::Byte;
use clap::Parser;
use errors::CopyError;
use stats::Accumulator;

fn main() -> Result<(), CopyError> {
    let cli = Args::parse();

    if !cli.src.exists() {
        return Err(CopyError::SourceNotFound(cli.src));
    }

    let mut accumulator = Accumulator::default();
    if !cli.src.is_dir() {
        return Err(CopyError::NotFaster);
    }

    if cli.skip && cli.overwrite {
        return Err(CopyError::Other("Cannot have both skip and overwrite set.".to_string()));
    }

    let opts = Arc::new(cli);

    let threads = opts.threads.unwrap_or_else(default_thread_count);
    println!("Starting copy with {} threads", threads);

    // If this list is very large, it could use quite a lot of memory.
    // TODO: Allow max queue size and run search and copy in parallel.
    let queue = search_dir(&opts.src, &mut accumulator, threads, opts.clone()).unwrap();
    copy_queue(
        queue,
        opts.src.clone(),
        opts.dst.clone(),
        &mut accumulator,
        threads,
        opts.clone(),
    )?;

    Ok(())
}

/// Get the number of available cores as a default, or `2` if we cannot determine the number of cores available.
///
/// # Notes
/// This isn't strictly number of available cores as implementation varies by platform. See [`std::thread::available_parallelism`]
/// for more details.
///
/// Prints an error and warning if we're falling back due to being unable to determine core count.
fn default_thread_count() -> usize {
    match std::thread::available_parallelism() {
        Ok(num) => usize::from(num),
        Err(e) => {
            println!("Error: {:?}", e);
            println!("Warning: could not determine available core count. Defaulting to 2 threads.");
            2
        }
    }
}

struct ResultInfo {
    path: PathBuf,
    metadata: Metadata,
}

enum SearchResult {
    File(ResultInfo),
    Directory(ResultInfo),
    Done,
}

fn search_dir(
    src: &Path,
    accumulator: &mut Accumulator,
    threads: usize,
    opts: Arc<Args>,
) -> std::io::Result<VecDeque<SearchResult>> {
    let start = Instant::now();

    let (result_sender, result_receiver) = channel();

    let mut path_senders = Vec::with_capacity(threads);

    let mut thread_handles = Vec::with_capacity(threads);

    for _ in 0..threads {
        let (path_sender, path_receiver) = channel();
        path_senders.push(path_sender);
        let result_sender = result_sender.clone();
        let handle = std::thread::spawn(move || {
            search(path_receiver, result_sender);
        });

        thread_handles.push(handle);
    }

    if path_senders[0].send(src.to_path_buf()).is_err() {
        return Err(std::io::ErrorKind::Interrupted.into());
    }

    let mut pending = 1;
    let mut sender_idx = 1;

    let mut last_time = Instant::now();

    let mut queue = VecDeque::new();

    while pending > 0 {
        match result_receiver.recv().unwrap() {
            SearchResult::File(file_result) => {
                *accumulator += Accumulator::found(1, file_result.metadata.len());
                queue.push_back(SearchResult::File(file_result));
            }
            SearchResult::Directory(dir_result) => {
                pending += 1;
                path_senders[sender_idx]
                    .send(dir_result.path.clone())
                    .unwrap();
                sender_idx += 1;
                if sender_idx == path_senders.len() {
                    sender_idx = 0;
                }
                queue.push_back(SearchResult::Directory(dir_result));
            }
            SearchResult::Done => pending -= 1,
        }

        if opts.progress {
            let now = Instant::now();
            if now.duration_since(last_time).as_secs() >= 5 {
                println!(
                    "Found {} files so far. Total size: {} bytes",
                    accumulator.file_count_found,
                    Byte::from_bytes(accumulator.byte_count_found as u128)
                        .get_appropriate_unit(false)
                );
                last_time = now;
            }
        }
    }
    let search_finish = Instant::now();

    println!(
        "Found {} files. Total size: {} bytes",
        accumulator.file_count_found,
        Byte::from_bytes(accumulator.byte_count_found as u128).get_appropriate_unit(false)
    );

    println!(
        "Search finished in {:.3} seconds",
        search_finish.duration_since(start).as_secs_f32()
    );

    for sender in path_senders {
        drop(sender);
    }

    for thread in thread_handles {
        thread.join().unwrap();
    }

    Ok(queue)
}

fn search(rx: Receiver<PathBuf>, found: Sender<SearchResult>) {
    for path in rx {
        for item in std::fs::read_dir(path).unwrap() {
            let entry = item.unwrap();
            let metadata = entry.metadata().unwrap();
            let path = entry.path();
            if path.is_dir() {
                let result_info = ResultInfo { path, metadata };
                found.send(SearchResult::Directory(result_info)).unwrap();
            } else {
                let result_info = ResultInfo { path, metadata };
                found.send(SearchResult::File(result_info)).unwrap();
            }
        }
        found.send(SearchResult::Done).unwrap();
    }
}

struct ThreadReady(usize, Accumulator);

fn copy_thread(
    thread_id: usize,
    copy_base: PathBuf,
    dest_base: PathBuf,
    request_sender: Sender<Result<ThreadReady, CopyError>>,
    path_receiver: Receiver<SearchResult>,
    opts: Arc<Args>,
) {
    if request_sender
        .send(Ok(ThreadReady(thread_id, Accumulator::default())))
        .is_ok()
    {
        for result in path_receiver {
            let accumulator = match result {
                SearchResult::File(file_result) => {
                    let relative = file_result.path.strip_prefix(&copy_base).unwrap();
                    let new_path = dest_base.join(relative);
                    let mut skipped: bool = false;
                    if !file_result.path.exists() {
                        println!("File found during scan no longer exists: {:?}", file_result.path.as_os_str());
                        skipped = true;
                    }
                    if new_path.exists() {
                        if !opts.skip && !opts.overwrite {
                            // If many files exist at the destination, all of the threads will hit this condition, but the first one to hit it will
                            // succeed with this send. Ignore the result and just kill the thread either way.
                            let _ = request_sender.send(Err(CopyError::CannotOverwrite(new_path)));
                            return;
                        }
                        if opts.skip {
                            skipped = true;
                        }
                    }
                    if !skipped {
                        let dir = new_path.parent().unwrap();
                        if !dir.exists() {
                            if let Err(err) = std::fs::DirBuilder::new().recursive(true).create(dir) {
                                let _ = request_sender
                                    .send(Err(CopyError::DirectoryCreationFailed(err.to_string())));
                                return;
                            }
                        }
                        match std::fs::copy(&file_result.path, &new_path) {
                            Ok(_) => {}
                            Err(err) if err.kind() == ErrorKind::PermissionDenied => {
                                let _ = request_sender
                                    .send(Err(CopyError::AccessDenied((file_result.path, new_path))));
                                return;
                            }
                            Err(err) => {
                                let _ =
                                    request_sender.send(Err(CopyError::Other(err.kind().to_string())));
                                return;
                            }
                        }
                        Accumulator::copies(1, file_result.metadata.len())
                    } else {
                        Accumulator::skips(1, file_result.metadata.len())
                    }
                }
                SearchResult::Directory(dir_result) => {
                    let relative = dir_result.path.strip_prefix(&copy_base).unwrap();
                    let new_path = dest_base.join(relative);
                    if let Err(err) = std::fs::DirBuilder::new().recursive(true).create(new_path) {
                        let _ = request_sender
                            .send(Err(CopyError::DirectoryCreationFailed(err.to_string())));
                        return;
                    }
                    Accumulator::default()
                }
                SearchResult::Done => Accumulator::default(),
            };

            // This only fails if the main thread is exiting so we can let the thread die.
            if request_sender
                .send(Ok(ThreadReady(thread_id, accumulator)))
                .is_err()
            {
                return;
            }
        }
    }
}

fn copy_queue(
    mut queue: VecDeque<SearchResult>,
    copy_base: PathBuf,
    dest_base: PathBuf,
    accumulator: &mut Accumulator,
    threads: usize,
    opts: Arc<Args>,
) -> Result<(), CopyError> {
    let copy_start = Instant::now();
    let (request_sender, request_receiver) = channel();
    let mut path_senders = Vec::with_capacity(threads);
    let mut thread_handles = Vec::with_capacity(threads);

    for idx in 0..threads {
        let request_sender = request_sender.clone();
        let (path_sender, path_receiver) = channel();
        path_senders.push(path_sender);
        let copy_base = copy_base.clone();
        let dest_base = dest_base.clone();
        let opts = opts.clone();

        let handle = std::thread::spawn(move || {
            copy_thread(
                idx,
                copy_base,
                dest_base,
                request_sender,
                path_receiver,
                opts,
            )
        });
        thread_handles.push(handle);
    }

    let mut idle = 0;

    let mut last_print = copy_start;

    for rq in request_receiver {
        let rq = rq?;
        if let Some(p) = queue.pop_front() {
            path_senders[rq.0].send(p).unwrap();
            *accumulator += rq.1;
        } else {
            *accumulator += rq.1;
            idle += 1;
        }

        if opts.progress {
            let now = Instant::now();
            if now.duration_since(last_print).as_secs() >= 5 {
                last_print = now;
                println!(
                    "Files: {} / {} ({:.2}%). Bytes: {} / {} ({:.2}%)",
                    accumulator.file_count_copied,
                    accumulator.file_count_found,
                    accumulator.file_count_copied as f64 / accumulator.file_count_found as f64
                        * 100.0,
                    Byte::from_bytes(accumulator.byte_count_copied as u128)
                        .get_appropriate_unit(false),
                    Byte::from_bytes(accumulator.byte_count_found as u128)
                        .get_appropriate_unit(false),
                    accumulator.byte_count_copied as f64 / accumulator.byte_count_found as f64
                        * 100.0
                )
            }
        }

        if idle == threads {
            break;
        }
    }

    let seconds = Instant::now().duration_since(copy_start).as_secs_f64();
    println!(
        "Finished copy of {} files ({}) in {:.2} seconds, (~{}/s), {} files ({}) skipped.",
        accumulator.file_count_copied,
        Byte::from_bytes(accumulator.byte_count_copied as u128).get_appropriate_unit(false),
        seconds,
        Byte::from_bytes((accumulator.byte_count_copied as f64 / seconds) as u128)
            .get_appropriate_unit(false),
        accumulator.file_count_skipped,
        Byte::from_bytes(accumulator.byte_count_skipped as u128).get_appropriate_unit(false),
    );

    for sender in path_senders {
        drop(sender);
    }

    for handle in thread_handles {
        handle.join().unwrap();
    }

    Ok(())
}
