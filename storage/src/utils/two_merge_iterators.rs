use crate::key::Key;
use crate::utils::storage_iterator::StorageIterator;

pub struct TwoMergeIterator<A: StorageIterator, B: StorageIterator> {
    a: A,
    b: B,
    choose_a: bool,
    current_value_a: bool,
    first_iteration: bool,
}

impl<A: StorageIterator, B: StorageIterator> TwoMergeIterator<A, B> {
    pub fn new(mut a: A, mut b: B) -> TwoMergeIterator<A, B> {
        a.next();
        b.next();
        let choose_a = Self::choose_a(&a, &b);
        let current_value_a = choose_a;

        TwoMergeIterator { a, b, choose_a, current_value_a, first_iteration: true }
    }

    fn choose_a(a: &A, b: &B) -> bool {
        if !a.has_next() {
            return false;
        }
        if !b.has_next() {
            return true;
        }

        a.key() > b.key()
    }

    fn skip_b_duplicates(&mut self) {
        while self.a.has_next() && self.b.has_next() && self.a.key() == self.b.key() {
            self.b.next();
        }
    }
}

impl<A: StorageIterator, B: StorageIterator> StorageIterator for TwoMergeIterator<A, B> {
    fn next(&mut self) -> bool {
        //As StorageIterator::new calls next(), we dont want to call it twice from the users code
        if self.first_iteration {
            self.first_iteration = false;
            return self.has_next();
        }

        let mut advanced: bool = false;

        if self.choose_a {
            advanced = self.a.next();
            self.current_value_a = true;
        } else { //Choose b
            advanced = self.b.next();
            self.current_value_a = false;
        }

        self.skip_b_duplicates();
        self.choose_a = Self::choose_a(&self.a, &self.b);

        advanced
    }

    fn has_next(&self) -> bool {
        self.a.has_next() || self.b.has_next()
    }

    fn key(&self) -> &Key {
        if self.current_value_a {
            self.a.key()
        } else {
            self.b.key()
        }
    }

    fn value(&self) -> &[u8] {
        if self.current_value_a {
            self.a.value()
        } else {
            self.b.value()
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use crate::key;
    use crate::memtables::memtable::{MemTable, MemtableIterator};
    use crate::transactions::transaction::Transaction;
    use crate::utils::storage_iterator::StorageIterator;
    use crate::utils::two_merge_iterators::TwoMergeIterator;

    #[test]
    fn two_merge_iterator() {
        let memtable1 = Arc::new(MemTable::create_mock(Arc::new(shared::SimpleDbOptions::default()), 0).unwrap());
        memtable1.set_active();
        memtable1.set(&Transaction::none(), "a", &vec![1]);
        memtable1.set(&Transaction::none(), "b", &vec![2]);
        memtable1.set(&Transaction::none(), "d", &vec![4]);

        let memtable2 = Arc::new(MemTable::create_mock(Arc::new(shared::SimpleDbOptions::default()), 0).unwrap());
        memtable2.set_active();
        memtable1.set(&Transaction::none(), "a", &vec![1]);
        memtable1.set(&Transaction::none(), "c", &vec![3]);
        memtable1.set(&Transaction::none(), "d", &vec![4]);
        memtable1.set(&Transaction::none(), "f", &vec![5]);

        let mut two_merge_iterators = TwoMergeIterator::new(
            MemtableIterator::create(&memtable1, &Transaction::none()),
            MemtableIterator::create(&memtable2, &Transaction::none()),
        );

        assert!(two_merge_iterators.has_next());
        two_merge_iterators.next();
        assert!(two_merge_iterators.key().eq(&key::new("a", 0)));
        assert!(two_merge_iterators.value().eq(&vec![1]));

        assert!(two_merge_iterators.has_next());
        two_merge_iterators.next();
        assert!(two_merge_iterators.key().eq(&key::new("b", 0)));
        assert!(two_merge_iterators.value().eq(&vec![2]));

        assert!(two_merge_iterators.has_next());
        two_merge_iterators.next();
        assert!(two_merge_iterators.key().eq(&key::new("c", 0)));
        assert!(two_merge_iterators.value().eq(&vec![3]));

        assert!(two_merge_iterators.has_next());
        two_merge_iterators.next();
        assert!(two_merge_iterators.key().eq(&key::new("d", 0)));
        assert!(two_merge_iterators.value().eq(&vec![4]));

        assert!(two_merge_iterators.has_next());
        two_merge_iterators.next();
        assert!(two_merge_iterators.key().eq(&key::new("f", 0)));
        assert!(two_merge_iterators.value().eq(&vec![5]));

        assert!(!two_merge_iterators.has_next());
    }
}