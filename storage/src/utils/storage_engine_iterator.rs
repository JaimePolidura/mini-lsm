use std::collections::VecDeque;
use std::sync::Arc;
use bytes::Bytes;
use shared::StorageValueMergeResult;
use crate::key::Key;
use crate::transactions::transaction::Transaction;
use crate::transactions::transaction_manager::TransactionManager;
use crate::utils::storage_iterator::StorageIterator;

//This is the iterator that will be exposed to users of the storage engine:
//This iterator merges the values by the merger function defined in SimpleDbOptions
//And commits the transaction when the iterator is dropped, if the itrerator was created in "standalone" mode
// which means when the transaction was created only for the iterator, (for example: call to Storage::scan_from or Storage::scan_all)
pub struct StorageEngineItertor<I: StorageIterator> {
    options: Arc<shared::SimpleDbOptions>,
    inner_iterator: I,

    entries_to_return: VecDeque<(Key, Bytes)>, //We use VecDequeue so that we can pop from index 0

    current_value: Option<Bytes>,
    current_key: Option<Key>,

    transaction_manager: Option<Arc<TransactionManager>>,
    transaction: Option<Transaction>,

    seeked_key: Option<Bytes>,
    is_seeked_key_inclusive: bool,

    last_entry_returned: bool,
}

impl<I: StorageIterator> StorageEngineItertor<I> {
    //For efficiency, you should call seek_key() in the inner iterator
    pub fn create_seeked_key(
        options: &Arc<shared::SimpleDbOptions>,
        iterator: I,
        seeked_key: Bytes,
        inclusive: bool,
    ) -> StorageEngineItertor<I> {
        let mut iterator = Self::create(options, iterator);
        iterator.is_seeked_key_inclusive = inclusive;
        iterator.seeked_key = Some(seeked_key);
        iterator
    }

    pub fn create(options: &Arc<shared::SimpleDbOptions>, mut iterator: I) -> StorageEngineItertor<I> {
        if iterator.has_next() {
            iterator.next();
        }

        StorageEngineItertor {
            entries_to_return: VecDeque::new(),
            transaction_manager: None,
            last_entry_returned: false,
            inner_iterator: iterator,
            options: options.clone(),
            current_value: None,
            current_key: None,
            transaction: None,
            seeked_key: None,
            is_seeked_key_inclusive: false
        }
    }

    pub fn set_transaction_standalone(
        &mut self,
        transaction_manager: &Arc<TransactionManager>,
        transaction: Transaction
    ) {
        self.transaction_manager = Some(transaction_manager.clone());
        self.transaction = Some(transaction);
    }

    fn find_entries(&mut self) -> bool {
        self.entries_to_return.push_back((
            self.inner_iterator.key().clone(),
            Bytes::copy_from_slice(self.inner_iterator.value()))
        );

        let current_key_bytes = Bytes::copy_from_slice(self.inner_iterator.key().as_bytes());

        let has_next = self.inner_iterator.has_next();

        while has_next {
            self.inner_iterator.next();
            let next_key = self.inner_iterator.key();

            if next_key.bytes_eq_bytes(&current_key_bytes) {
                self.entries_to_return.push_back((
                    self.inner_iterator.key().clone(),
                    Bytes::copy_from_slice(self.inner_iterator.value()))
                );
            } else {
                break
            }
        }

        if !has_next {
            self.last_entry_returned = true;
        }

        self.merge_entry_values();

        true
    }

    fn merge_entry_values(&mut self) {
        if self.options.storage_value_merger.is_none() || self.entries_to_return.len() <= 1 {
            //No merger function specified, no neecesity to merge values
            return;
        }

        let mut prev_merged_value: Option<(Key, Bytes)> = None;
        let merge_fn = self.options.storage_value_merger.unwrap();

        while let Some((next_key, next_value)) = self.entries_to_return.pop_front() {
            match prev_merged_value.take() {
                Some((_, previous_merged_value)) => {
                    match merge_fn(&previous_merged_value, &next_value) {
                        StorageValueMergeResult::Ok(merged_value) => prev_merged_value = Some((next_key, merged_value)),
                        StorageValueMergeResult::DiscardPrevious => prev_merged_value = Some((next_key, next_value)),
                    }
                },
                None => {
                    prev_merged_value = Some((next_key, next_value))
                }
            }
        }

        let (final_key, final_value) = prev_merged_value.take().unwrap();
        self.entries_to_return.push_front((final_key, final_value));
    }

    fn do_do_next(&mut self) -> bool {
        if self.last_entry_returned {
            return false;
        }
        if self.entries_to_return.is_empty() && !self.find_entries() {
            return false;
        }

        let (next_key, next_value) = self.entries_to_return.pop_front().unwrap();
        self.current_value = Some(next_value);
        self.current_key = Some(next_key);

        true
    }
}

impl<I: StorageIterator> StorageIterator for StorageEngineItertor<I> {
    fn next(&mut self) -> bool {
        while self.do_do_next() {
            if let Some(seeked_key) = self.seeked_key.as_ref() {
                let is_inbound = if self.is_seeked_key_inclusive {
                    self.key().bytes().ge(seeked_key)
                } else {
                    self.key().bytes().gt(seeked_key)
                };

                if is_inbound {
                    self.seeked_key.take();
                    return true;
                }
            } else {
                return true;
            }
        }

        false
    }

    fn has_next(&self) -> bool {
        todo!()
    }

    fn key(&self) -> &Key {
        self.current_key.as_ref().unwrap()
    }

    fn value(&self) -> &[u8] {
        self.current_value.as_ref().unwrap()
    }
}

impl<I: StorageIterator> Drop for StorageEngineItertor<I> {
    fn drop(&mut self) {
        if let Some(transaction_manager) = self.transaction_manager.as_ref() {
            transaction_manager.commit(self.transaction.as_ref().unwrap());
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use bytes::Bytes;
    use shared::StorageValueMergeResult;
    use crate::key;
    use crate::memtables::memtable::{MemTable};
    use crate::memtables::memtable_iterator::MemtableIterator;
    use crate::transactions::transaction::Transaction;
    use crate::utils::storage_engine_iterator::StorageEngineItertor;
    use crate::utils::storage_iterator::StorageIterator;

    #[test]
    fn iterator_no_merger_fn() {
        let options = Arc::new(shared::SimpleDbOptions::default());
        let memtable = Arc::new(MemTable::create_mock(Arc::new(shared::SimpleDbOptions::default()), 0)
            .unwrap());
        memtable.set(&transaction(10), Bytes::from("aa"), &vec![1]);
        memtable.set(&transaction(1), Bytes::from("alberto"), &vec![2]);
        memtable.set(&transaction(3), Bytes::from("alberto"), &vec![4]);
        memtable.set(&transaction(1), Bytes::from("gonchi"), &vec![5]);
        memtable.set(&transaction(5), Bytes::from("javier"), &vec![6]);
        memtable.set(&transaction(5), Bytes::from("jaime"), &vec![8]);
        memtable.set(&transaction(1), Bytes::from("wili"), &vec![9]);

        let mut iterator = StorageEngineItertor::create(
            &options,
            MemtableIterator::create(&memtable, &Transaction::none())
        );

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("aa", 10)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("alberto", 1)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("alberto", 3)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("gonchi", 1)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("jaime", 5)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("javier", 5)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("wili", 1)));

        // assert!(!iterator.next());
    }

    fn merge_values(a: &Bytes, b: &Bytes) -> StorageValueMergeResult {
        let a_vec = a.to_vec();
        let b_vec = b.to_vec();

        if b_vec[0] != 10 {
            StorageValueMergeResult::Ok(Bytes::from(vec![a_vec[0] + b_vec[0]]))
        } else {
            StorageValueMergeResult::DiscardPrevious
        }
    }

    #[test]
    fn iterator_merger_fn() {
        let options = shared::start_simpledb_options_builder_from(&shared::SimpleDbOptions::default())
            .storage_value_merger(|a, b| merge_values(a, b))
            .build_arc();

        let memtable = Arc::new(MemTable::create_mock(options.clone(), 0).unwrap());
        memtable.set(&transaction(10), Bytes::from("aa"), &vec![1]);
        memtable.set(&transaction(1), Bytes::from("alberto"), &vec![1]);
        memtable.set(&transaction(3), Bytes::from("alberto"), &vec![1]);
        memtable.set(&transaction(4), Bytes::from("alberto"), &vec![1]);
        memtable.set(&transaction(1), Bytes::from("gonchi"), &vec![1]);
        memtable.set(&transaction(5), Bytes::from("javier"), &vec![1]);
        memtable.set(&transaction(5), Bytes::from("jaime"), &vec![1]);
        memtable.set(&transaction(1), Bytes::from("wili"), &vec![1]);
        memtable.set(&transaction(1), Bytes::from("wili"), &vec![10]); //10 Equivalent of tombstone
        memtable.set(&transaction(1), Bytes::from("wili"), &vec![2]);

        let mut iterator = StorageEngineItertor::create(
            &options,
            MemtableIterator::create(&memtable, &Transaction::none())
        );

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("aa", 10)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("alberto", 4)));
        assert!(iterator.value().eq(&vec![3]));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("gonchi", 1)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("jaime", 5)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("javier", 5)));

        assert!(iterator.next());
        assert!(iterator.key().eq(&key::create_from_str("wili", 1)));
        assert!(iterator.value().eq(&vec![2]));

        assert!(!iterator.next());
    }

    fn transaction(txn_id: shared::TxnId) -> Transaction {
        let mut transaction = Transaction::none();
        transaction.txn_id = txn_id;
        transaction
    }
}