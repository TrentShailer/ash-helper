/// Number of bytes to align the start of each slice to.
const ALIGNMENT: u64 = 64;

/// Calculate the start and end points of a slice of a buffer.
/// * Slices are aligned to 64 byte boundaries.
/// * Elements are aligned to `alignment`.
pub fn calc_slice<T>(previous_end: u64, alignment: u64, count: u64) -> (u64, u64) {
    let start_padding = (ALIGNMENT - previous_end % ALIGNMENT) % ALIGNMENT;

    let start = previous_end + start_padding;

    let element_padding = (alignment - start % alignment) % alignment;
    let element_size = size_of::<T>() as u64 + element_padding;

    let end = start + element_size * count;

    (start, end)
}
