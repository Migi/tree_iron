#[cfg(test)]
mod tests {
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::ops::{Deref, DerefMut};

	// using AtomicUsize mostly to prevent compiler optimizations
	struct Checked<T> {
		val: T,
		dropcnt: AtomicUsize,
		active_refs: AtomicUsize,
		active_ref_muts: AtomicUsize
	}

	impl<T> Drop for Checked<T> {
		fn drop(&mut self) {
			let old_dropcnt = self.dropcnt.fetch_add(1, Ordering::SeqCst);
			if old_dropcnt != 0 {
				panic!("Double drop detected! Dropped {} times already!", old_dropcnt);
			}
		}
	}

	impl<T> Checked<T> {
		fn new(val: T) -> Self {
			Checked {
				val,
				dropcnt: AtomicUsize::new(0),
				active_refs: AtomicUsize::new(0),
				active_ref_muts: AtomicUsize::new(0)
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
			CheckedRef {
				r: self
			}
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
				panic!("Accessing mutably while active_ref_muts = {} > 0", active_ref_muts);
			}
			CheckedRefMut {
				r: self
			}
		}
	}

	struct CheckedRef<'a,T> {
		r: &'a Checked<T>
	}

	impl<'a,T> Drop for CheckedRef<'a,T> {
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
				panic!("Dropping ref while active_ref_muts = {} > 0", active_ref_muts);
			}
		}
	}

	impl<'a,T> Deref for CheckedRef<'a,T> {
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

	struct CheckedRefMut<'a,T> {
		r: &'a mut Checked<T>
	}

	impl<'a,T> Drop for CheckedRefMut<'a,T> {
		fn drop(&mut self) {
			let dropcnt = self.r.dropcnt.load(Ordering::SeqCst);
			if dropcnt > 0 {
				panic!("Dropping mutable ref while dropcnt = {} > 0", dropcnt);
			}
			let active_refs = self.r.active_refs.load(Ordering::SeqCst);
			if active_refs > 0 {
				panic!("Dropping mutable ref while active_refs = {} > 0", active_refs);
			}
			let active_ref_muts = self.r.active_ref_muts.fetch_sub(1, Ordering::SeqCst);
			if active_ref_muts == 0 {
				panic!("Dropping mutable ref while active_ref_muts == 0");
			}
		}
	}

	impl<'a,T> Deref for CheckedRefMut<'a,T> {
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

	impl<'a,T> DerefMut for CheckedRefMut<'a,T> {
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

	fn build_tree() -> Immutree<Checked<i32>> {
		let mut tree = Immutree::new();
		tree.build_root_node(Checked::new(2), |node| {
			node.build_child(Checked::new(10), |node| {
				node.add_leaf_child(Checked::new(11));
				node.add_leaf_child(Checked::new(12));
				node.add_leaf_child(Checked::new(13));
			});
			node.add_leaf_child(Checked::new(20));
			node.build_child(Checked::new(30), |node| {
				node.add_leaf_child(Checked::new(31));
				node.add_leaf_child(Checked::new(32));
				node.add_leaf_child(Checked::new(33));
			});
		});
		tree
	}

	#[test]
	fn test_iter_tree() {
		let tree = build_tree();

		let mut iter = tree.iter();
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
		assert!(iter.next().is_none());
	}

	#[test]
	fn test_iter_tree_mut_but_only_read() {
		let mut tree = build_tree();

		let mut iter = tree.iter_mut();
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
		assert!(iter.next().is_none());
	}

	#[test]
	fn test_iter_tree_mut() {
		let mut tree = build_tree();

		let mut iter = tree.iter_mut();
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
		assert!(iter.next().is_none());
	}

	#[test]
	fn test_drain_tree() {
		let tree = build_tree();
		let mut drain = tree.drain();
		let mut iter = drain.drain_all();
		let (val,sub_children) = iter.next().unwrap().into_val_and_children();
		assert_eq!(*val.get(), 2);
		{
			let mut children = sub_children;
			let (val,sub_children) = children.next().unwrap().into_val_and_children();
			assert_eq!(*val.get(), 10);
			{
				let mut children = sub_children;
				// 11
				let (node,mut sub_children) = children.next().unwrap().into_val_and_children();
				assert_eq!(*node.get(), 11);
				assert!(sub_children.next().is_none());
				// 12
				let (node,mut sub_children) = children.next().unwrap().into_val_and_children();
				assert_eq!(*node.get(), 12);
				assert!(sub_children.next().is_none());
				// 13
				let (node,mut sub_children) = children.next().unwrap().into_val_and_children();
				assert_eq!(*node.get(), 13);
				assert!(sub_children.next().is_none());
				// end
				assert!(children.next().is_none());
			}
			let (val,mut sub_children) = children.next().unwrap().into_val_and_children();
			assert_eq!(*val.get(), 20);
			assert!(sub_children.next().is_none());
			let (val,sub_children) = children.next().unwrap().into_val_and_children();
			assert_eq!(*val.get(), 30);
			{
				let mut children = sub_children;
				// 31
				let (node,mut sub_children) = children.next().unwrap().into_val_and_children();
				assert_eq!(*node.get(), 31);
				assert!(sub_children.next().is_none());
				// 32
				let (node,mut sub_children) = children.next().unwrap().into_val_and_children();
				assert_eq!(*node.get(), 32);
				assert!(sub_children.next().is_none());
				// 33
				let (node,mut sub_children) = children.next().unwrap().into_val_and_children();
				assert_eq!(*node.get(), 33);
				assert!(sub_children.next().is_none());
				// end
				assert!(children.next().is_none());
			}
			assert!(children.next().is_none());
		}
		assert!(iter.next().is_none());
	}
}
