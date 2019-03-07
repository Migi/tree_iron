use crate::*;

use std::iter::{Iterator, ExactSizeIterator};

struct ExactSize<T> {
	val: T,
	num_children: usize
}

pub struct ExactSizeTreeStore<T> {
    store: TreeStore<ExactSize<T>>,
	num_trees: usize
}

impl<T> ExactSizeTreeStore<T> {
    pub fn new() -> ExactSizeTreeStore<T> {
        ExactSizeTreeStore {
            store: TreeStore::new(),
			num_trees: 0
        }
    }

    pub fn with_capacity(capacity: usize) -> ExactSizeTreeStore<T> {
        ExactSizeTreeStore {
            store: TreeStore::with_capacity(capacity),
			num_trees: 0
        }
    }

    pub fn add_tree<R>(
        &mut self,
        initial_val: T,
        node_builder_cb: impl FnOnce(ExactSizeNodeBuilder<T>) -> R,
    ) -> R {
		self.num_trees += 1;
		
		let exact_size = ExactSize {
			val: initial_val,
			num_children: 0
		};
		self.store.add_tree(exact_size, move |node_builder| {
			node_builder_cb(ExactSizeNodeBuilder {
				node_builder
			})
		})
    }

	pub fn num_trees(&self) -> usize {
		self.num_trees
	}

    pub fn iter_trees(&self) -> ExactSizeNodeIter<T> {
        ExactSizeNodeIter {
            iter: self.store.iter_trees(),
			len: self.num_trees()
        }
    }

    pub fn iter_trees_mut(&mut self) -> ExactSizeNodeIterMut<T> {
		let len = self.num_trees();
        ExactSizeNodeIterMut {
            iter: self.store.iter_trees_mut(),
			len
        }
    }

    pub fn drain_trees(self) -> ExactSizeTreeDrain<T> {
		let num_trees = self.num_trees();
        ExactSizeTreeDrain {
            drain: self.store.drain_trees(),
			num_trees
        }
    }
}

/// test
pub struct ExactSizeNodeBuilder<'a, T> {
	node_builder: NodeBuilder<'a, ExactSize<T>>
}

impl<'a, T> ExactSizeNodeBuilder<'a, T> {
    pub fn val(&self) -> &T {
        &self.node_builder.val().val
    }

    pub fn val_mut(&mut self) -> &mut T {
        &mut self.node_builder.val_mut().val
    }

	pub fn num_children_so_far(&self) -> usize {
        self.node_builder.val().num_children
	}

    pub fn add_leaf_child(&mut self, val: T) {
        self.add_child(val, |_| {});
    }

    pub fn add_child<R>(
        &mut self,
        initial_val: T,
        child_builder_cb: impl FnOnce(ExactSizeNodeBuilder<T>) -> R,
    ) -> R {
		self.node_builder.val_mut().num_children += 1;

		let exact_size = ExactSize {
			val: initial_val,
			num_children: 0
		};
		self.node_builder.add_child(exact_size, move |node_builder| {
			child_builder_cb(ExactSizeNodeBuilder {
				node_builder
			})
		})
    }
}

/// test
pub struct ExactSizeNodeIter<'t, T> {
    iter: NodeIter<'t, ExactSize<T>>,
	len: usize
}

/// test
pub struct ExactSizeNodeRef<'t, T> {
    node_ref: NodeRef<'t, ExactSize<T>>
}

impl<'t, T> Iterator for ExactSizeNodeIter<'t, T> {
    type Item = ExactSizeNodeRef<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
		match self.iter.next() {
			Some(node_ref) => {
				debug_assert!(self.len > 0);
				self.len -= 1;
				Some(ExactSizeNodeRef {
					node_ref
				})
			},
			None => {
				debug_assert_eq!(self.len, 0);
				None
			}
		}
    }

	fn size_hint(&self) -> (usize,Option<usize>) {
		(self.len,Some(self.len))
	}
}

impl<'t, T> ExactSizeIterator for ExactSizeNodeIter<'t, T> {
	fn len(&self) -> usize {
		self.len
	}
}

impl<'t, T> ExactSizeNodeRef<'t, T> {
    pub fn children(self) -> ExactSizeNodeIter<'t, T> {
		let len = self.num_children();
        ExactSizeNodeIter {
			iter: self.node_ref.children(),
			len
		}
    }

    pub fn val(&self) -> &T {
		&self.node_ref.val().val
    }

	pub fn num_children(&self) -> usize {
		self.node_ref.val().num_children
    }
}

/// test
pub struct ExactSizeNodeIterMut<'t, T> {
    iter: NodeIterMut<'t, ExactSize<T>>,
	len: usize
}

/// test
pub struct ExactSizeNodeRefMut<'t, T> {
    node_ref: NodeRefMut<'t, ExactSize<T>>
}

impl<'t, T> Iterator for ExactSizeNodeIterMut<'t, T> {
    type Item = ExactSizeNodeRefMut<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
		match self.iter.next() {
			Some(node_ref) => {
				debug_assert!(self.len > 0);
				self.len -= 1;
				Some(ExactSizeNodeRefMut {
					node_ref
				})
			},
			None => {
				debug_assert_eq!(self.len, 0);
				None
			}
		}
    }

	fn size_hint(&self) -> (usize,Option<usize>) {
		(self.len,Some(self.len))
	}
}

impl<'t, T> ExactSizeIterator for ExactSizeNodeIterMut<'t, T> {
	fn len(&self) -> usize {
		self.len
	}
}

impl<'t, T> ExactSizeNodeRefMut<'t, T> {
    pub fn children(self) -> ExactSizeNodeIterMut<'t, T> {
		let len = self.num_children();
        ExactSizeNodeIterMut {
			iter: self.node_ref.children(),
			len
		}
    }

    pub fn val(&self) -> &T {
		&self.node_ref.val().val
    }

    pub fn val_mut(&mut self) -> &mut T {
		&mut self.node_ref.val_mut().val
    }

	pub fn num_children(&self) -> usize {
		self.node_ref.val().num_children
    }
}

pub struct ExactSizeTreeDrain<T> {
    drain: TreeDrain<ExactSize<T>>,
	num_trees: usize
}

impl<T> ExactSizeTreeDrain<T> {
    pub fn drain_all(&mut self) -> ExactSizeNodeListDrain<T> {
        ExactSizeNodeListDrain {
			drain: self.drain.drain_all(),
			len: self.num_trees
		}
    }
}

/// test
pub struct ExactSizeNodeListDrain<'t, T> {
    drain: NodeListDrain<'t, ExactSize<T>>,
	len: usize
}

/// test
pub struct ExactSizeNodeDrain<'t, T> {
    node: NodeDrain<'t, ExactSize<T>>
}

impl<'t, T> Iterator for ExactSizeNodeListDrain<'t, T> {
    type Item = ExactSizeNodeDrain<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
		match self.drain.next() {
			Some(node) => {
				debug_assert!(self.len > 0);
				self.len -= 1;
				Some(ExactSizeNodeDrain {
					node
				})
			},
			None => {
				debug_assert_eq!(self.len, 0);
				None
			}
		}
    }

	fn size_hint(&self) -> (usize,Option<usize>) {
		(self.len,Some(self.len))
	}
}

impl<'t, T> ExactSizeIterator for ExactSizeNodeListDrain<'t, T> {
	fn len(&self) -> usize {
		self.len
	}
}

impl<'t, T> ExactSizeNodeDrain<'t, T> {
    pub fn into_val_and_children(self) -> (T, ExactSizeNodeListDrain<'t, T>) {
		let len = self.num_children();
		let (val, children) = self.node.into_val_and_children();
        (val.val, ExactSizeNodeListDrain {
			drain: children,
			len
		})
    }

    pub fn val(&self) -> &T {
		&self.node.val().val
    }

	pub fn num_children(&self) -> usize {
		self.node.val().num_children
    }
}
