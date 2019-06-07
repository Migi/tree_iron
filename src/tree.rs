use crate::*;

use std::mem::ManuallyDrop;

/// test
pub struct IronedTree<T> {
    forest:  IronedForest<T>,
    root_val: T
}

impl<T> IronedTree<T> {
    pub fn new(root_val: T) -> IronedTree<T> {
        IronedTree {
			forest: IronedForest::new(),
			root_val
		}
    }

    pub fn with_capacity(root_val: T) -> IronedTree<T> {
        IronedTree {
			forest: IronedForest::with_capacity(),
			root_val
		}
    }

    pub fn build_child_of_root<R>(
        &mut self,
        initial_val: T,
        node_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
    ) -> R {
        node_builder_cb(self.add_child(initial_val))
    }

    pub fn add_child_of_root(&mut self, initial_val: T) -> NodeBuilder<T> {
        self.forest.add_tree(initial_val)
    }

    pub fn iter_children_of_root(&self) -> NodeIter<T> {
        self.forest.iter_trees()
    }

    pub fn iter_children_of_root_mut(&mut self) -> NodeIterMut<T> {
        self.forest.iter_trees_mut()
    }

    pub fn drain(self) -> RootNodeDrain<T> {
        RootNodeDrain {
			val: self.root_val,
			children: self.forest.drain_trees()
		}
    }

    /// Read-only view of the raw data.
    pub fn raw_data(&self) -> &Vec<ManuallyDrop<NodeData<T>>> {
        self.forest.raw_data()
    }

    pub fn tot_num_nodes(&self) -> usize {
        self.forest.tot_num_nodes() + 1
    }
}

pub struct RootNodeDrain<T> {
	val: T,
    children: TreeDrain<T>,
}

impl<'t, T> RootNodeDrain<'t, T> {
    pub fn into_val_and_children(mut self) -> (T, NodeListDrain<'t, T>) {
        (self.val, self.children)
    }

    pub fn val(&self) -> &T {
        &self.val
    }

    pub fn val_mut(&mut self) -> &mut T {
        &mut self.val
    }

    pub fn num_descendants_incl_self(&self) -> usize {
        self.children().remaining_subtrees_size() + 1
    }

    pub fn num_descendants_excl_self(&self) -> usize {
        self.children().remaining_subtrees_size()
    }
}
