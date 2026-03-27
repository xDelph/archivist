use db::ThreadCardRow;
use std::collections::HashMap;

pub(crate) type ThreadCardKey = (String, String);

pub(crate) fn build_thread_card_lookup(
    rows: Vec<ThreadCardRow>,
) -> HashMap<ThreadCardKey, ThreadCardRow> {
    rows.into_iter()
        .map(|row| ((row.channel_id.clone(), row.root_ts.clone()), row))
        .collect()
}

pub(crate) fn lookup_thread_card<'a>(
    lookup: &'a HashMap<ThreadCardKey, ThreadCardRow>,
    channel_id: &str,
    root_ts: &str,
) -> Option<&'a ThreadCardRow> {
    lookup.get(&(channel_id.to_owned(), root_ts.to_owned()))
}
