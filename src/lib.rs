//! # quickcdc
//! `quickcdc` is a fast content defined chunker.
//! * For some background information, see [AE: An Asymmetric Extremum Content Defined Chunking Algorithm](https://ieeexplore.ieee.org/document/7218510) by Yucheng Zhang.
//! * Modification(s):
//!   * User may provide salt, introducing entropy / cutpoint variation (i.e. files re-processed with different salt values will produce different cutpoints).
//!   * Warp forward (reduced window size), skipping some unnecessary processing that happens before minimum chunk size is reached.
//!
//! This should be faster than many CDC algorithms (anecdotal performance: 2GB/s on an amd1950x with an NVMe drive), but faster alternatives exist.
//! * For more information, see [FastCDC](https://www.usenix.org/system/files/conference/atc16/atc16-paper-xia.pdf)
//!
//! NOTE: This implementation performs much faster when built with `--release`.
//!
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]

extern crate rand;

use rand::prelude::*;
use std::f64::consts;
use std::mem;

const SIZEOF_U64: usize = mem::size_of::<u64>();

#[derive(Debug)]
pub struct Chunker<'a> {
    slice: &'a [u8],
    window_size: usize,
    max_chunksize: usize,
    min_chunksize: usize,
    salt: u64,
    bytes_processed: usize,
    bytes_remaining: usize,
}

#[derive(Debug)]
pub enum ChunkerError {
    InsufficientMaxSize,
    InsufficientTargetSize,
}

impl<'a> Chunker<'a> {
    /// Given a {slice, target size, max_size, salt}, supply an iterable struct that produces chunked slices.
    ///
    /// # Examples
    /// ```
    /// use quickcdc;
    /// use rand::Rng;
    ///
    /// let mut rng = rand::thread_rng();
    /// let mut sample = [0u8; 1024];
    /// rng.fill(&mut sample[..]);
    /// let target_size = 64;
    /// let max_chunksize = 128;
    /// let salt = 15222894464462204665;
    ///
    /// let chunker = quickcdc::Chunker::with_params(&sample[..], target_size, max_chunksize, salt).unwrap();
    /// for x in chunker {
    ///     println!("{}", x.len());
    /// }
    ///
    /// ```
    pub fn with_params(
        slice: &[u8],
        target_chunksize_bytes: usize,
        max_chunksize_bytes: usize,
        salt: u64,
    ) -> Result<Chunker, ChunkerError> {
        if 2 * target_chunksize_bytes > max_chunksize_bytes {
            return Err(ChunkerError::InsufficientMaxSize);
        }

        if target_chunksize_bytes < 64 {
            return Err(ChunkerError::InsufficientTargetSize);
        }

        let target_window_size = (target_chunksize_bytes as f64 / (consts::E - 1.0)) as usize;
        let my_window_size = (target_window_size as f64 * 0.56) as usize;
        let min_chunksize = target_chunksize_bytes - target_window_size;
        let chunker: Chunker = Chunker {
            slice,
            window_size: my_window_size,
            salt,
            max_chunksize: max_chunksize_bytes,
            min_chunksize,
            bytes_processed: 0,
            bytes_remaining: slice.len(),
        };
        Ok(chunker)
    }

    /// Return a good (random) salt value.
    pub fn get_random_salt() -> u64 {
        let mut rng = rand::thread_rng();
        rng.next_u64()
    }
}

/// Returns the next content-defined chunk.
impl<'a> Iterator for Chunker<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.bytes_remaining == 0 {
            return None;
        }

        let next_slice = next_chunked_slice(
            &self.slice[(self.bytes_processed)..],
            self.window_size,
            self.min_chunksize,
            self.max_chunksize,
            self.salt,
        );
        self.bytes_processed += next_slice.len();
        self.bytes_remaining -= next_slice.len();

        Some(next_slice)
    }
}

/// Return the next content-defined slice.
fn next_chunked_slice(
    remaining: &[u8],
    window_size: usize,
    min_chunksize: usize,
    max_chunksize: usize,
    salt: u64,
) -> &[u8] {
    let remaining_bytes_length = remaining.len();

    // under minimum chunk size remaining
    if remaining_bytes_length <= min_chunksize + window_size {
        return &remaining[..remaining_bytes_length];
    }

    let mut marker_position = 0;
    let end_index = remaining_bytes_length - SIZEOF_U64;

    // Warp forward to avoid unnecessary processing
    for i in min_chunksize..end_index {
        // Max chunksize reached, force a cutpoint.
        // This generally happens when processing data that doesn't change (e.g. sparse files / all zeros).
        if i == max_chunksize {
            return &remaining[..i];
        }

        // Recast a pair of u64 pointers, to be used for comparison.
        // Since 'i' never iterates beyond slice (i.e. remaining_bytes_length - SIZEOF_U64),
        // we never dereference anything beyond the end of our slice.
        let current_as_u64 = &remaining[i] as *const u8 as *const u64;
        let marker_as_u64 = &remaining[marker_position] as *const u8 as *const u64;

        // Update marker position, if necessary
        if !swapped_salted_isgt(current_as_u64, marker_as_u64, salt) {
            marker_position = i;
            continue;
        }

        // End of window reached without a new marker position, force a cutpoint
        if i == marker_position + window_size {
            return &remaining[..i];
        }
    }

    // force a cutpoint
    let cutpoint = if max_chunksize < remaining_bytes_length {
        max_chunksize
    } else {
        remaining_bytes_length
    };
    &remaining[..cutpoint]
}

/// Utility Function: Compare pointers to two 64-bit portions of data.
///
/// It does the following:
/// * Dereferences each pointer into a u64 value.
/// * Byte-swaps each value, and XOR the result with supplied salt.
/// * Compare swapped+salted values, return comparison result.
///
/// De-referencing the pointers is an unsafe operation.  As long as the pointers do not extend beyond
/// the end of the slice being chunked, this function will not result in undefined behavior.
#[inline]
fn swapped_salted_isgt(first: *const u64, second: *const u64, salt: u64) -> bool {
    let compare_first = unsafe { (*first).swap_bytes() } ^ salt;
    let compare_second = unsafe { (*second).swap_bytes() } ^ salt;
    if compare_first > compare_second {
        return true;
    }
    false
}
