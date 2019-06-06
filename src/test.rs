#[cfg(test)]
mod tests {
    use std::ops::{Deref, DerefMut};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CheckedTest {
        num_undropped: AtomicUsize,
    }

    impl CheckedTest {
        fn new() -> CheckedTest {
            CheckedTest {
                num_undropped: AtomicUsize::new(0),
            }
        }

        fn num_undropped(&self) -> usize {
            self.num_undropped.load(Ordering::SeqCst)
        }
    }

    // using AtomicUsize mostly to prevent compiler optimizations
    struct Checked<T> {
        val: T,
        dropcnt: AtomicUsize,
        active_refs: AtomicUsize,
        active_ref_muts: AtomicUsize,
        test: Arc<CheckedTest>,
    }

    impl<T> Drop for Checked<T> {
        fn drop(&mut self) {
            let old_dropcnt = self.dropcnt.fetch_add(1, Ordering::SeqCst);
            if old_dropcnt != 0 {
                panic!(
                    "Double drop detected! Dropped {} times already!",
                    old_dropcnt
                );
            }
            let old_num_undropped = self.test.num_undropped.fetch_sub(1, Ordering::SeqCst);
            if old_num_undropped == 0 {
                panic!("Dropping Checked<T> while num_undropped == 0!");
            }
        }
    }

    impl<T> Checked<T> {
        fn new(val: T, test: Arc<CheckedTest>) -> Self {
            test.num_undropped.fetch_add(1, Ordering::SeqCst);
            Checked {
                val,
                dropcnt: AtomicUsize::new(0),
                active_refs: AtomicUsize::new(0),
                active_ref_muts: AtomicUsize::new(0),
                test,
            }
        }

        fn get(&self) -> CheckedRef<T> {
            let dropcnt = self.dropcnt.load(Ordering::SeqCst);
            if dropcnt > 0 {
                panic!("Accessing while dropcnt = {} > 0", dropcnt);
            }
            self.active_refs.fetch_add(1, Ordering::SeqCst);
            let active_ref_muts = self.active_ref_muts.load(Ordering::SeqCst);
            if active_ref_muts > 0 {
                panic!("Accessing while active_ref_muts = {} > 0", active_ref_muts);
            }
            CheckedRef { r: self }
        }

        fn get_mut(&mut self) -> CheckedRefMut<T> {
            let dropcnt = self.dropcnt.load(Ordering::SeqCst);
            if dropcnt > 0 {
                panic!("Accessing mutably while dropcnt = {} > 0", dropcnt);
            }
            let active_refs = self.active_refs.load(Ordering::SeqCst);
            if active_refs > 0 {
                panic!("Accessing mutably while active_refs = {} > 0", active_refs);
            }
            let active_ref_muts = self.active_ref_muts.fetch_add(1, Ordering::SeqCst);
            if active_ref_muts > 0 {
                panic!(
                    "Accessing mutably while active_ref_muts = {} > 0",
                    active_ref_muts
                );
            }
            CheckedRefMut { r: self }
        }
    }

    struct CheckedRef<'a, T> {
        r: &'a Checked<T>,
    }

    impl<'a, T> Drop for CheckedRef<'a, T> {
        fn drop(&mut self) {
            let dropcnt = self.r.dropcnt.load(Ordering::SeqCst);
            if dropcnt > 0 {
                panic!("Dropping ref while dropcnt = {} > 0", dropcnt);
            }
            let active_refs = self.r.active_refs.fetch_sub(1, Ordering::SeqCst);
            if active_refs == 0 {
                panic!("Dropping ref while active_refs == 0");
            }
            let active_ref_muts = self.r.active_ref_muts.load(Ordering::SeqCst);
            if active_ref_muts > 0 {
                panic!(
                    "Dropping ref while active_ref_muts = {} > 0",
                    active_ref_muts
                );
            }
        }
    }

    impl<'a, T> Deref for CheckedRef<'a, T> {
        type Target = T;

        fn deref(&self) -> &T {
            let dropcnt = self.r.dropcnt.load(Ordering::SeqCst);
            if dropcnt > 0 {
                panic!("Dereffing ref while dropcnt = {} > 0", dropcnt);
            }
            let active_refs = self.r.active_refs.load(Ordering::SeqCst);
            if active_refs == 0 {
                panic!("Dereffing while active_refs == 0");
            }
            let active_ref_muts = self.r.active_ref_muts.load(Ordering::SeqCst);
            if active_ref_muts > 0 {
                panic!("Dereffing while active_ref_muts = {} > 0", active_ref_muts);
            }
            &self.r.val
        }
    }

    struct CheckedRefMut<'a, T> {
        r: &'a mut Checked<T>,
    }

    impl<'a, T> Drop for CheckedRefMut<'a, T> {
        fn drop(&mut self) {
            let dropcnt = self.r.dropcnt.load(Ordering::SeqCst);
            if dropcnt > 0 {
                panic!("Dropping mutable ref while dropcnt = {} > 0", dropcnt);
            }
            let active_refs = self.r.active_refs.load(Ordering::SeqCst);
            if active_refs > 0 {
                panic!(
                    "Dropping mutable ref while active_refs = {} > 0",
                    active_refs
                );
            }
            let active_ref_muts = self.r.active_ref_muts.fetch_sub(1, Ordering::SeqCst);
            if active_ref_muts == 0 {
                panic!("Dropping mutable ref while active_ref_muts == 0");
            }
        }
    }

    impl<'a, T> Deref for CheckedRefMut<'a, T> {
        type Target = T;

        fn deref(&self) -> &T {
            let dropcnt = self.r.dropcnt.load(Ordering::SeqCst);
            if dropcnt > 0 {
                panic!("Dereffing mutably while dropcnt = {} > 0", dropcnt);
            }
            let active_refs = self.r.active_refs.load(Ordering::SeqCst);
            if active_refs > 0 {
                panic!("Dereffing mutably while active_refs = {} > 0", active_refs);
            }
            let active_ref_muts = self.r.active_ref_muts.load(Ordering::SeqCst);
            if active_ref_muts == 0 {
                panic!("Dereffing mutably while active_ref_muts == 0");
            }
            &self.r.val
        }
    }

    impl<'a, T> DerefMut for CheckedRefMut<'a, T> {
        fn deref_mut(&mut self) -> &mut T {
            let dropcnt = self.r.dropcnt.load(Ordering::SeqCst);
            if dropcnt > 0 {
                panic!("Dereffing mutably while dropcnt = {} > 0", dropcnt);
            }
            let active_refs = self.r.active_refs.load(Ordering::SeqCst);
            if active_refs > 0 {
                panic!("Dereffing mutably while active_refs = {} > 0", active_refs);
            }
            let active_ref_muts = self.r.active_ref_muts.load(Ordering::SeqCst);
            if active_ref_muts == 0 {
                panic!("Dereffing mutably while active_ref_muts == 0");
            }
            &mut self.r.val
        }
    }

    use crate::*;

    fn build_store(test: Arc<CheckedTest>) -> TreeStore<Checked<i32>> {
        let mut store = TreeStore::new();
        store.build_tree(Checked::new(1, test.clone()), |mut node| {
            node.build_child(Checked::new(8, test.clone()), |mut node| {
                node.add_child(Checked::new(11, test.clone()));
                node.add_child(Checked::new(12, test.clone()));
                *node.val_mut() = Checked::new(9, test.clone());
                node.add_child(Checked::new(13, test.clone()));
                *node.val_mut() = Checked::new(10, test.clone());
            });
            node.add_child(Checked::new(20, test.clone()));
            node.build_child(Checked::new(30, test.clone()), |mut node| {
                node.add_child(Checked::new(31, test.clone()));
                node.add_child(Checked::new(32, test.clone()));
                node.add_child(Checked::new(33, test.clone()));
            });
            *node.val_mut() = Checked::new(2, test.clone());
        });
        store.build_tree(Checked::new(3, test.clone()), |mut node| {
            node.add_child(Checked::new(10, test.clone()));
            node.build_child(Checked::new(20, test.clone()), |mut node| {
                node.add_child(Checked::new(21, test.clone()));
                node.add_child(Checked::new(22, test.clone()));
                node.add_child(Checked::new(23, test.clone()));
            });
            node.add_child(Checked::new(30, test.clone()));
        });
        store
    }

    #[test]
    fn test_iter() {
        let test = Arc::new(CheckedTest::new());
        {
            let store = build_store(test.clone());

            let mut iter = store.iter_trees();
            let node = iter.next().unwrap();
            assert_eq!(*node.val().get(), 2);
            {
                let mut children = node.children();
                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 10);
                {
                    let mut children = node.children();
                    // 11
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 11);
                    assert!(node.children().next().is_none());
                    // 12
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 12);
                    assert!(node.children().next().is_none());
                    // 13
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 13);
                    assert!(node.children().next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 20);
                assert!(node.children().next().is_none());

                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 30);
                {
                    let mut children = node.children();
                    // 31
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 31);
                    assert!(node.children().next().is_none());
                    // 32
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 32);
                    assert!(node.children().next().is_none());
                    // 33
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 33);
                    assert!(node.children().next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                assert!(children.next().is_none());
            }
            let node = iter.next().unwrap();
            assert_eq!(*node.val().get(), 3);
            {
                let mut children = node.children();
                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 10);
                assert!(node.children().next().is_none());

                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 20);
                {
                    let mut children = node.children();
                    // 21
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 21);
                    assert!(node.children().next().is_none());
                    // 22
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 22);
                    assert!(node.children().next().is_none());
                    // 23
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 23);
                    assert!(node.children().next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 30);
                assert!(node.children().next().is_none());

                assert!(children.next().is_none());
            }
            assert!(iter.next().is_none());
        }
        assert_eq!(test.num_undropped(), 0);
    }

    #[test]
    fn test_iter_mut_but_only_read() {
        let test = Arc::new(CheckedTest::new());
        {
            let mut store = build_store(test.clone());

            let mut iter = store.iter_trees_mut();
            let node = iter.next().unwrap();
            assert_eq!(*node.val().get(), 2);
            {
                let mut children = node.children();
                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 10);
                {
                    let mut children = node.children();
                    // 11
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 11);
                    assert!(node.children().next().is_none());
                    // 12
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 12);
                    assert!(node.children().next().is_none());
                    // 13
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 13);
                    assert!(node.children().next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 20);
                assert!(node.children().next().is_none());

                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 30);
                {
                    let mut children = node.children();
                    // 31
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 31);
                    assert!(node.children().next().is_none());
                    // 32
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 32);
                    assert!(node.children().next().is_none());
                    // 33
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 33);
                    assert!(node.children().next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                assert!(children.next().is_none());
            }
            let node = iter.next().unwrap();
            assert_eq!(*node.val().get(), 3);
            {
                let mut children = node.children();
                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 10);
                assert!(node.children().next().is_none());

                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 20);
                {
                    let mut children = node.children();
                    // 21
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 21);
                    assert!(node.children().next().is_none());
                    // 22
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 22);
                    assert!(node.children().next().is_none());
                    // 23
                    let node = children.next().unwrap();
                    assert_eq!(*node.val().get(), 23);
                    assert!(node.children().next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                let node = children.next().unwrap();
                assert_eq!(*node.val().get(), 30);
                assert!(node.children().next().is_none());
                
                assert!(children.next().is_none());
            }
            assert!(iter.next().is_none());
        }
        assert_eq!(test.num_undropped(), 0);
    }

    #[test]
    fn test_iter_mut() {
        let test = Arc::new(CheckedTest::new());
        {
            let mut store = build_store(test.clone());

            let mut iter = store.iter_trees_mut();
            let mut node = iter.next().unwrap();
            assert_eq!(*node.val_mut().get_mut(), 2);
            {
                let mut children = node.children();
                let mut node = children.next().unwrap();
                assert_eq!(*node.val_mut().get_mut(), 10);
                {
                    let mut children = node.children();
                    // 11
                    let mut node = children.next().unwrap();
                    assert_eq!(*node.val_mut().get_mut(), 11);
                    assert!(node.children().next().is_none());
                    // 12
                    let mut node = children.next().unwrap();
                    assert_eq!(*node.val_mut().get_mut(), 12);
                    assert!(node.children().next().is_none());
                    // 13
                    let mut node = children.next().unwrap();
                    assert_eq!(*node.val_mut().get_mut(), 13);
                    assert!(node.children().next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                let mut node = children.next().unwrap();
                assert_eq!(*node.val_mut().get_mut(), 20);
                assert!(node.children().next().is_none());

                let mut node = children.next().unwrap();
                assert_eq!(*node.val_mut().get_mut(), 30);
                {
                    let mut children = node.children();
                    // 31
                    let mut node = children.next().unwrap();
                    assert_eq!(*node.val_mut().get_mut(), 31);
                    assert!(node.children().next().is_none());
                    // 32
                    let mut node = children.next().unwrap();
                    assert_eq!(*node.val_mut().get_mut(), 32);
                    assert!(node.children().next().is_none());
                    // 33
                    let mut node = children.next().unwrap();
                    assert_eq!(*node.val_mut().get_mut(), 33);
                    assert!(node.children().next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                assert!(children.next().is_none());
            }
            let mut node = iter.next().unwrap();
            assert_eq!(*node.val_mut().get_mut(), 3);
            {
                let mut children = node.children();
                let mut node = children.next().unwrap();
                assert_eq!(*node.val_mut().get_mut(), 10);
                assert!(node.children().next().is_none());

                let mut node = children.next().unwrap();
                assert_eq!(*node.val_mut().get_mut(), 20);
                {
                    let mut children = node.children();
                    // 21
                    let mut node = children.next().unwrap();
                    assert_eq!(*node.val_mut().get_mut(), 21);
                    assert!(node.children().next().is_none());
                    // 22
                    let mut node = children.next().unwrap();
                    assert_eq!(*node.val_mut().get_mut(), 22);
                    assert!(node.children().next().is_none());
                    // 23
                    let mut node = children.next().unwrap();
                    assert_eq!(*node.val_mut().get_mut(), 23);
                    assert!(node.children().next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                let mut node = children.next().unwrap();
                assert_eq!(*node.val_mut().get_mut(), 30);
                assert!(node.children().next().is_none());
                
                assert!(children.next().is_none());
            }
            assert!(iter.next().is_none());
        }
        assert_eq!(test.num_undropped(), 0);
    }

    #[test]
    fn test_drain() {
        let test = Arc::new(CheckedTest::new());
        {
            let store = build_store(test.clone());

            let mut drain = store.drain_trees();
            let mut iter = drain.drain_all();
            let (val, sub_children) = iter.next().unwrap().into_val_and_children();
            assert_eq!(*val.get(), 2);
            {
                let mut children = sub_children;
                let (val, sub_children) = children.next().unwrap().into_val_and_children();
                assert_eq!(*val.get(), 10);
                {
                    let mut children = sub_children;
                    // 11
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 11);
                    assert!(sub_children.next().is_none());
                    // 12
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 12);
                    assert!(sub_children.next().is_none());
                    // 13
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 13);
                    assert!(sub_children.next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                let (val, mut sub_children) = children.next().unwrap().into_val_and_children();
                assert_eq!(*val.get(), 20);
                assert!(sub_children.next().is_none());
                let (val, sub_children) = children.next().unwrap().into_val_and_children();
                assert_eq!(*val.get(), 30);
                {
                    let mut children = sub_children;
                    // 31
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 31);
                    assert!(sub_children.next().is_none());
                    // 32
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 32);
                    assert!(sub_children.next().is_none());
                    // 33
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 33);
                    assert!(sub_children.next().is_none());
                    // end
                    assert!(children.next().is_none());
                }
                assert!(children.next().is_none());
            }
            let (val, sub_children) = iter.next().unwrap().into_val_and_children();
            assert_eq!(*val.get(), 3);
            {
                let mut children = sub_children;

                let (val, mut sub_children) = children.next().unwrap().into_val_and_children();
                assert_eq!(*val.get(), 10);
                assert!(sub_children.next().is_none());

                let (val, sub_children) = children.next().unwrap().into_val_and_children();
                assert_eq!(*val.get(), 20);
                {
                    let mut children = sub_children;
                    // 21
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 21);
                    assert!(sub_children.next().is_none());
                    // 22
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 22);
                    assert!(sub_children.next().is_none());
                    // 23
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 23);
                    assert!(sub_children.next().is_none());
                    // end
                    assert!(children.next().is_none());
                }

                let (val, mut sub_children) = children.next().unwrap().into_val_and_children();
                assert_eq!(*val.get(), 30);
                assert!(sub_children.next().is_none());
                
                assert!(children.next().is_none());
            }
            assert!(iter.next().is_none());
        }
        assert_eq!(test.num_undropped(), 0);
    }

    #[test]
    fn test_drain_create_only() {
        let test = Arc::new(CheckedTest::new());
        {
            let store = build_store(test.clone());
            let _drain = store.drain_trees();
        }
        assert_eq!(test.num_undropped(), 0);
    }

    #[test]
    fn test_drain_halfway() {
        let test = Arc::new(CheckedTest::new());
        {
            let store = build_store(test.clone());

            let mut drain = store.drain_trees();
            let mut iter = drain.drain_all();
            let (val, sub_children) = iter.next().unwrap().into_val_and_children();
            assert_eq!(*val.get(), 2);
            {
                let mut children = sub_children;
                let (val, sub_children) = children.next().unwrap().into_val_and_children();
                assert_eq!(*val.get(), 10);
                {
                    let mut children = sub_children;
                    // 11
                    let (node, _sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 11);
                    // 12
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 12);
                    assert!(sub_children.next().is_none());
                }
                let _ = children.next();
            }
        }
        assert_eq!(test.num_undropped(), 0);
    }

    #[test]
    fn test_drain_only_last_half() {
        let test = Arc::new(CheckedTest::new());
        {
            let store = build_store(test.clone());

            let mut drain = store.drain_trees();
            let mut iter = drain.drain_all();
            let (val, sub_children) = iter.next().unwrap().into_val_and_children();
            assert_eq!(*val.get(), 2);
            {
                let mut children = sub_children;
                let _ = children.next();
                let _ = children.next();
                let (val, sub_children) = children.next().unwrap().into_val_and_children();
                assert_eq!(*val.get(), 30);
                {
                    let mut children = sub_children;
                    // 31
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 31);
                    assert!(sub_children.next().is_none());
                    // 32
                    let _ = children.next();
                }
                assert!(children.next().is_none());
            }
            let (val, sub_children) = iter.next().unwrap().into_val_and_children();
            assert_eq!(*val.get(), 3);
            {
                let mut children = sub_children;
                let _ = children.next();
                let (val, sub_children) = children.next().unwrap().into_val_and_children();
                assert_eq!(*val.get(), 20);
                {
                    let mut children = sub_children;
                    // 21
                    let (node, mut sub_children) = children.next().unwrap().into_val_and_children();
                    assert_eq!(*node.get(), 21);
                    assert!(sub_children.next().is_none());
                    // 22
                    let _ = children.next();
                }
                assert_eq!(*children.next().unwrap().val().get(), 30);
                assert!(children.next().is_none());
            }
            assert!(iter.next().is_none());
        }
        assert_eq!(test.num_undropped(), 0);
    }
}
