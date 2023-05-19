#[derive(Debug, Clone)]
pub struct OngoingTransfer {
    pub file_offset: u64,
    pub file_size: f64,
    pub file_path: String,
}
