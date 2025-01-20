/// Number of bytes to align the start of each slice to.
const ALIGNMENT: u64 = 16;

/// Calculate the start and end points of a slice of a buffer.
/// * Slices are aligned to 16 byte boundaries.
/// * `T` must have no padding between instances of `T`.
pub fn calc_slice<T>(previous_end: u64, count: u64) -> (u64, u64) {
    let padding = (ALIGNMENT - previous_end % ALIGNMENT) % ALIGNMENT;

    let start = previous_end + padding;
    let end = start + size_of::<T>() as u64 * count;

    (start, end)
}
