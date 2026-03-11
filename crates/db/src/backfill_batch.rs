#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BackfillBatchStats {
    pub messages_inserted: usize,
    pub messages_duplicate: usize,
    pub reactions_inserted: usize,
    pub reactions_duplicate: usize,
    pub refreshed_threads: usize,
}
