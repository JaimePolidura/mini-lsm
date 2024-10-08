use std::fs::DirEntry;

pub(crate) fn is_sstable_file(file: &DirEntry) -> bool {
    file.file_name().to_str().unwrap().starts_with("sst-")
}

pub(crate) fn to_sstable_file_name(sstable_id: shared::SSTableId) -> String {
    let result = format!("sst-{}", sstable_id);
    result
}

pub(crate) fn extract_sstable_id_from_file(file: &DirEntry) -> Result<shared::SSTableId, ()> {
    shared::extract_number_from_file_name(file, "-")
}
