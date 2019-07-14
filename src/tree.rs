use std::convert::{From, TryFrom, AsRef};
use crate::*;

/// test
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct PackedTree<T> {
    forest: PackedForest<T>,
}

impl<T> PackedTree<T> {
    pub fn new(root_val: T, node_builder_cb: impl FnOnce(&mut NodeBuilder<T>)) -> PackedTree<T> {
        let mut forest = PackedForest::new();
        forest.build_tree(root_val, node_builder_cb);
        PackedTree { forest }
    }

    pub fn new_by_ret_val(node_builder_cb: impl FnOnce(&mut NodeBuilder<T>) -> T) -> PackedTree<T> {
        let mut forest = PackedForest::new();
        forest.build_tree_by_ret_val(node_builder_cb);
        PackedTree { forest }
    }

    pub fn root(&self) -> NodeRef<T> {
        self.forest.iter_trees().next().unwrap()
    }

    pub fn root_mut(&mut self) -> NodeRefMut<T> {
        self.forest.iter_trees_mut().next().unwrap()
    }

    pub fn drain(self) -> PackedTreeDrain<T> {
        PackedTreeDrain {
            forest: self.forest
        }
    }

    pub fn iter_flattened<'a>(
        &'a self,
    ) -> std::iter::Map<std::slice::Iter<'a, NodeData<T>>, impl FnMut(&'a NodeData<T>) -> &'a T>
    {
        self.forest.iter_flattened()
    }

    pub fn iter_flattened_mut<'a>(
        &'a mut self,
    ) -> std::iter::Map<
        std::slice::IterMut<'a, NodeData<T>>,
        impl FnMut(&'a mut NodeData<T>) -> &'a mut T,
    > {
        self.forest.iter_flattened_mut()
    }

    /// Read-only view of the raw data.
    pub fn raw_data(&self) -> &Vec<NodeData<T>> {
        self.forest.raw_data()
    }

    pub fn tot_num_nodes(&self) -> usize {
        self.forest.tot_num_nodes()
    }
}

impl<T> TryFrom<PackedForest<T>> for PackedTree<T> {
    type Error = ();
    fn try_from(forest: PackedForest<T>) -> Result<Self, Self::Error> {
        if forest.iter_trees().count() == 1 {
            Ok(PackedTree { forest })
        } else {
            Err(())
        }
    }
}

impl<T> AsRef<PackedForest<T>> for PackedTree<T> {
    fn as_ref(&self) -> &PackedForest<T> {
        &self.forest
    }
}

impl<T> From<PackedTree<T>> for PackedForest<T> {
    fn from(tree: PackedTree<T>) -> Self {
        tree.forest
    }
}

/// test
pub struct PackedTreeDrain<T> {
    forest: PackedForest<T>,
}

impl<T> PackedTreeDrain<T> {
    pub fn drain_root(&mut self) -> NodeDrain<T> {
        self.forest.drain_trees().next().unwrap()
    }

    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<std::vec::Drain<NodeData<T>>, impl FnMut(NodeData<T>) -> T> {
        self.forest.drain_flattened()
    }
}
