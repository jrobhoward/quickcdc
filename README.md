# quickcdc
## Summary
`quickcdc` is a fast content defined chunker for `&[u8]` slices.
* For some background information, see [AE: An Asymmetric Extremum Content Defined Chunking Algorithm](https://ieeexplore.ieee.org/document/7218510) by Yucheng Zhang.
* Modification(s):
  * User may provide salt, introducing entropy / cutpoint variation (i.e. files re-processed with different salt values will produce different cutpoints).
  * Warp forward (reduced window size), skipping some unnecessary processing that happens before minimum chunk size is reached.

This should be faster than many CDC algorithms (anecdotal performance: 2GB/s on an amd1950x with an NVMe drive), but faster alternatives exist.
* For more information, see [FastCDC](https://www.usenix.org/system/files/conference/atc16/atc16-paper-xia.pdf)

NOTE: This implementation performs much faster when built with `--release`.

## Example
```rust
use quickcdc;
use rand::Rng;

let mut rng = rand::thread_rng();
let mut sample = [0u8; 1024];
rng.fill(&mut sample[..]);
let target_size = 64;
let max_chunksize = 128;
let salt = 15222894464462204665;

let chunker = quickcdc::Chunker::with_params(&sample[..], target_size, max_chunksize, salt).unwrap();
for x in chunker {
    println!("{}", x.len());
}
```
