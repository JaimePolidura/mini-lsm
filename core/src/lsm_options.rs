use std::sync::Arc;
use crate::compaction::simple_leveled::SimpleLeveledCompactionOptions;
use crate::compaction::tiered::TieredCompactionOptions;

#[derive(Clone, Copy)]
pub enum CompactionStrategy {
    SimpleLeveled,
    Tiered,
}

#[derive(Clone)]
pub struct LsmOptions {
    pub simple_leveled_compaction_options: SimpleLeveledCompactionOptions,
    pub tiered_compaction_options: TieredCompactionOptions,
    pub compaction_strategy: CompactionStrategy,
    pub compaction_task_frequency_ms: usize,
    pub n_cached_blocks_per_sstable: usize,
    pub memtable_max_size_bytes: usize,
    pub max_memtables_inactive: usize,
    pub bloom_filter_n_entries: usize,
    pub block_size_bytes: usize,
    pub sst_size_bytes: usize,
    pub base_path: String,
}

impl Default for LsmOptions {
    fn default() -> Self {
        LsmOptions {
            simple_leveled_compaction_options: SimpleLeveledCompactionOptions::default(),
            tiered_compaction_options: TieredCompactionOptions::default(),
            compaction_strategy: CompactionStrategy::SimpleLeveled,
            compaction_task_frequency_ms: 100, //100ms
            memtable_max_size_bytes: 1048576, //1Mb
            bloom_filter_n_entries: 32768, //4kb of bloom filter so it fits in a page
            block_size_bytes: 4096, //4kb
            sst_size_bytes: 268435456, //256 MB ~ 64 blocks
            n_cached_blocks_per_sstable: 8, //Expect power of two
            max_memtables_inactive: 8,
            base_path: String::from("ignored"),
        }
    }
}

pub fn builder() -> LsmOptionsBuilder {
    LsmOptionsBuilder {
        lsm_options: LsmOptions::default()
    }
}

pub struct LsmOptionsBuilder {
    lsm_options: LsmOptions,
}

impl LsmOptionsBuilder {
    pub fn simple_leveled_compaction_options(&mut self, value: SimpleLeveledCompactionOptions) -> &mut LsmOptionsBuilder {
        self.lsm_options.simple_leveled_compaction_options = value;
        self
    }

    pub fn tiered_compaction_options(&mut self, value: TieredCompactionOptions) -> &mut LsmOptionsBuilder {
        self.lsm_options.tiered_compaction_options = value;
        self
    }

    pub fn compaction_strategy(&mut self, value: CompactionStrategy) -> &mut LsmOptionsBuilder {
        self.lsm_options.compaction_strategy = value;
        self
    }

    pub fn compaction_task_frequency_ms(&mut self, value: usize) -> &mut LsmOptionsBuilder {
        self.lsm_options.compaction_task_frequency_ms = value;
        self
    }

    pub fn n_cached_blocks_per_sstable(&mut self, value: usize) -> &mut LsmOptionsBuilder {
        self.lsm_options.n_cached_blocks_per_sstable = value;
        self
    }

    pub fn memtable_max_size_bytes(&mut self, value: usize) -> &mut LsmOptionsBuilder {
        self.lsm_options.memtable_max_size_bytes = value;
        self
    }

    pub fn max_memtables_inactive(&mut self, value: usize) -> &mut LsmOptionsBuilder {
        self.lsm_options.max_memtables_inactive = value;
        self
    }

    pub fn bloom_filter_n_entries(&mut self, value: usize) -> &mut LsmOptionsBuilder {
        self.lsm_options.bloom_filter_n_entries = value;
        self
    }

    pub fn block_size_bytes(&mut self, value: usize) -> &mut LsmOptionsBuilder {
        self.lsm_options.block_size_bytes = value;
        self
    }

    pub fn sst_size_bytes(&mut self, value: usize) -> &mut LsmOptionsBuilder {
        self.lsm_options.sst_size_bytes = value;
        self
    }

    pub fn base_path(&mut self, value: String) -> &mut LsmOptionsBuilder {
        self.lsm_options.base_path = value;
        self
    }

    pub fn build(&self) -> Arc<LsmOptions> {
        Arc::new(self.lsm_options.clone())
    }
}