pub fn get_batch<T>(
    apps: &[T],
    batch_size: usize,
    selected_index: usize,
) -> (usize, usize, usize, &[T]) {
    let total_batches = apps.len().div_ceil(batch_size).max(1);

    let active_batch_index = selected_index / batch_size;

    let active_batch_index = usize::min(active_batch_index, total_batches - 1);

    let start_index = active_batch_index * batch_size;
    let end_index = usize::min(start_index + batch_size, apps.len());

    let batch_slice = &apps[start_index..end_index];

    let selected_in_batch_index = selected_index - start_index;

    (
        active_batch_index + 1,
        total_batches,
        selected_in_batch_index,
        batch_slice,
    )
}

#[cfg(test)]
mod tests {
    use super::get_batch;

    #[test]
    fn pages_through_items() {
        let items: Vec<usize> = (0..16).collect();
        assert_eq!(get_batch(&items, 8, 0), (1, 2, 0, &items[0..8]));
        assert_eq!(get_batch(&items, 8, 9), (2, 2, 1, &items[8..16]));

        let items: Vec<usize> = (0..17).collect();
        assert_eq!(get_batch(&items, 8, 16), (3, 3, 0, &items[16..17]));
    }

    #[test]
    fn empty_list_is_one_empty_page() {
        let items: Vec<usize> = Vec::new();
        assert_eq!(get_batch(&items, 8, 0), (1, 1, 0, &items[..]));
    }
}
