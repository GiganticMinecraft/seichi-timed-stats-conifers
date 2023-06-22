use crate::structures_embedded_in_rdb::DiffPointId;

pub fn display_diff_point_ids_for_tracing<'a>(
    diff_point_ids: impl IntoIterator<Item = &'a DiffPointId>,
) -> String {
    let diff_point_ids = diff_point_ids
        .into_iter()
        .map(|id| id.0)
        .collect::<Vec<_>>();
    format!("{:?}", diff_point_ids)
}
