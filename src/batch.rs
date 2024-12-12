use crate::parser::App;

pub fn get_batch(
    apps: &Vec<App>,
    batch_size: usize,
    selected_index: usize,
) -> (usize, usize, usize, &[App]) {
    let total_batches = (apps.len() / batch_size) as usize + 1;

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
