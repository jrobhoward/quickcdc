extern crate quickcdc;
#[macro_use]
extern crate quickcheck;

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    #[test]
    fn chunker__processing_zeroed_array__always_returns_max_chunk_size() {
        let target_size = 64;
        let max_size = 1024;
        let zero_array = [0u8; 10240];

        let chunker =
            quickcdc::Chunker::with_params(&zero_array[..], target_size, max_size, 0).unwrap();

        for chunk in chunker {
            assert_eq!(chunk.len(), max_size);
        }
    }

    #[test]
    fn chunker__when_array_not_evently_divisible__returns_expected_remainder() {
        let target_size = 64;
        let max_size = 1024;
        let uneven_array = [0u8; 102401];

        let chunker =
            quickcdc::Chunker::with_params(&uneven_array[..], target_size, max_size, 0).unwrap();
        let mut peekable_chunker = chunker.peekable();

        while let Some(chunk) = peekable_chunker.next() {
            if peekable_chunker.peek().is_none() {
                assert_eq!(chunk.len(), 1);
            } else {
                assert_eq!(chunk.len(), max_size);
            }
        }
    }

    #[test]
    fn chunker__when_given_undersized_target_size__returns_error() {
        let target_size = 63; // 64 is minimum
        let max_size = 1024;
        let zero_array = [0u8; 10240];

        let chunker = quickcdc::Chunker::with_params(&zero_array[..], target_size, max_size, 0);

        assert_eq!(chunker.is_err(), true);
    }

    #[test]
    fn chunker__when_given_undersized_max_size__returns_error() {
        let target_size = 64;
        let max_size = 127; // must be at least 2 * target size
        let zero_array = [0u8; 10240];

        let chunker = quickcdc::Chunker::with_params(&zero_array[..], target_size, max_size, 0);

        assert_eq!(chunker.is_err(), true);
    }

    // Formatting appears to be broken within quickcheck macros...
    // `quickcheck_macros` (requires rust nightly) should fix this
    // For now, manually format:
    // 1.) Comment out quickcheck! line & it's closing counterpart
    // 2.) Run `cargo fmt`
    // 3.) Uncomment
    quickcheck! {
    fn chunker__given_any_salt__chunks_not_oversized(salt: u64, slice: Vec<u8>) -> bool {
        let target_size = 64;
        let max_size = 1024;
        let chunker = quickcdc::Chunker::with_params(&slice, target_size, max_size, salt).unwrap();
        for x in chunker {
            if x.len() > max_size {
                return false;
            }
        }
        true
    }

    fn chunker__given_same_salt__returns_same_result(salt: u64, slice: Vec<u8>) -> bool {
        let target_size = 64;
        let max_size = 1024;
        let chunker_one = quickcdc::Chunker::with_params(&slice, target_size, max_size, salt).unwrap();
        let chunker_two = quickcdc::Chunker::with_params(&slice, target_size, max_size, salt).unwrap();

        use std::collections::VecDeque;
        let result_one: VecDeque<&[u8]> = chunker_one.collect();
        let result_two: VecDeque<&[u8]> = chunker_two.collect();

        result_one == result_two
    }
    }
}
