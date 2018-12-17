// Use system malloc
use std::alloc::System;
#[global_allocator]
static GLOBAL: System = System;

extern crate memmap;
extern crate rand;
extern crate time;
extern crate walkdir;

use memmap::Mmap;
use quickcdc;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use time::PreciseTime;
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = env::args().collect();

    let walk_root = if let Some(path) = args.get(1) {
        path
    } else {
        println!("Usage: {} <path>", &args[0]);
        return;
    };

    let rng = quickcdc::Chunker::get_random_salt();
    let mut total_size: u64 = 0;
    let mut total_count: u64 = 0;
    let mut files_processed = 0;
    let mut paths_skipped = 0;

    println!("Processing files under path: {}", walk_root);
    let start_time = PreciseTime::now();
    for entry in WalkDir::new(walk_root) {
        let dir_entry = if entry.is_err() {
            paths_skipped += 1;
            continue;
        } else {
            entry.unwrap()
        };

        let mmap = match to_mmap(dir_entry.path()) {
            Err(_err) => {
                paths_skipped += 1;
                continue;
            }
            Ok(mmap) => {
                files_processed += 1;
                mmap
            }
        };

        let target_size = 128_000;
        let max_size = 524_288;
        let as_slice = mmap.as_ref();
        match quickcdc::Chunker::with_params(as_slice, target_size, max_size, rng) {
            Ok(chunker) => {
                for x in chunker {
                    total_size += x.len() as u64;
                    total_count += 1;
                }
            }
            Err(e) => println!("Unable to create new chunker {:?}", e),
        }
    }
    let end_time = PreciseTime::now();

    println!(
        "Duration: {} milliseconds",
        start_time.to(end_time).num_milliseconds()
    );
    println!("Files Processed: {}", files_processed);
    println!("Paths Skipped: {}", paths_skipped);
    println!("Chunks Processed: {}", total_count);
    if total_count > 0 {
        println!("Average Chunk Size: {}", total_size / total_count);
    }
    println!("Total Bytes Processed: {}", total_size);
}

fn to_mmap(path: &Path) -> io::Result<Mmap> {
    let metadata = fs::metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(io::ErrorKind::Other, "not a file"));
    }
    if metadata.len() == 0 {
        return Err(io::Error::new(io::ErrorKind::Other, "zero sized file"));
    }
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file) }?;
    Ok(mmap)
}
