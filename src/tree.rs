use crate::*;

/// test
pub struct IronedTree<T> {
    forest: IronedForest<T>,
    root_val: T,
}

impl<T> IronedTree<T> {
    pub fn new(root_val: T) -> IronedTree<T> {
        IronedTree {
            forest: IronedForest::new(),
            root_val,
        }
    }

    pub fn with_capacity(root_val: T, capacity: usize) -> IronedTree<T> {
        IronedTree {
            forest: IronedForest::with_capacity(capacity),
            root_val,
        }
    }

    pub fn build_child_of_root<R>(
        &mut self,
        initial_val: T,
        node_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
    ) -> R {
        node_builder_cb(self.add_child_of_root(initial_val))
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

    pub fn drain_root_node_and_children(self) -> (T,TreeDrain<T>) {
		(self.val, TreeDrain {
			forest: self.forest
		})
    }

    /// Read-only view of the raw data.
    pub fn raw_data(&self) -> &Vec<NodeData<T>> {
        self.forest.raw_data()
    }

    pub fn tot_num_nodes(&self) -> usize {
        self.forest.tot_num_nodes() + 1
    }
}

pub struct TreeDrain<T> {
    forest: IronedForest<T>,
}

impl<T> TreeDrain<T> {
    pub fn drain_children(&mut self) -> NodeListDrain<'_, T> {
        self.forest.drain_trees()
    }
}
