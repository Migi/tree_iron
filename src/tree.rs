use std::convert::{From, TryFrom, AsRef, AsMut};
use crate::*;

/// test
pub struct IronedTree<T> {
    forest: IronedForest<T>,
}

impl<T> IronedTree<T> {
    pub fn new(root_val: T, node_builder_cb: impl FnOnce(&mut NodeBuilder<T>)) -> IronedTree<T> {
        IronedTree::new_with_return_val(root_val, node_builder_cb).0
    }

    pub fn new_with_return_val<R>(
        root_val: T,
        node_builder_cb: impl FnOnce(&mut NodeBuilder<T>) -> R,
    ) -> (IronedTree<T>, R) {
        let mut forest = IronedForest::new();
        let ret = forest.build_tree(root_val, node_builder_cb);
        (IronedTree { forest }, ret)
    }

    pub fn new_with_capacity(
        root_val: T,
        node_builder_cb: impl FnOnce(&mut NodeBuilder<T>),
        capacity: usize,
    ) -> IronedTree<T> {
        IronedTree::new_with_capacity_and_return_val(root_val, node_builder_cb, capacity).0
    }

    pub fn new_with_capacity_and_return_val<R>(
        root_val: T,
        node_builder_cb: impl FnOnce(&mut NodeBuilder<T>) -> R,
        capacity: usize,
    ) -> (IronedTree<T>, R) {
        let mut forest = IronedForest::with_capacity(capacity);
        let ret = forest.build_tree(root_val, node_builder_cb);
        (IronedTree { forest }, ret)
    }

    pub fn root(&self) -> NodeRef<T> {
        self.forest.iter_trees().next().unwrap()
    }

    pub fn root_mut(&mut self) -> NodeRefMut<T> {
        self.forest.iter_trees_mut().next().unwrap()
    }

    pub fn drain_root(&mut self) -> NodeDrain<T> {
        self.forest.drain_trees().next().unwrap()
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

    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<std::vec::Drain<NodeData<T>>, impl FnMut(NodeData<T>) -> T> {
        self.forest.drain_flattened()
    }

    /// Read-only view of the raw data.
    pub fn raw_data(&self) -> &Vec<NodeData<T>> {
        self.forest.raw_data()
    }

    pub fn tot_num_nodes(&self) -> usize {
        self.forest.tot_num_nodes()
    }
}

impl<T> TryFrom<IronedForest<T>> for IronedTree<T> {
    type Error = ();
    fn try_from(forest: IronedForest<T>) -> Result<Self, Self::Error> {
        if forest.iter_trees().count() == 1 {
            Ok(IronedTree { forest })
        } else {
            Err(())
        }
    }
}

impl<T> AsRef<IronedForest<T>> for IronedTree<T> {
    fn as_ref(&self) -> &IronedForest<T> {
        &self.forest
    }
}

impl<T> AsMut<IronedForest<T>> for IronedTree<T> {
    fn as_mut(&mut self) -> &mut IronedForest<T> {
        &mut self.forest
    }
}

impl<T> From<IronedTree<T>> for IronedForest<T> {
    fn from(tree: IronedTree<T>) -> Self {
        tree.forest
    }
}
