/*use crate::*;

use std::convert::TryFrom;
use std::iter::{ExactSizeIterator, Iterator};

pub struct ExactSize<T> {
    val: T,
    num_children: usize,
}

impl<T> ExactSize<T> {
    pub fn val(&self) -> &T {
        &self.val
    }

    pub fn num_children(&self) -> usize {
        self.num_children
    }
}

pub struct ExactSizeIronedForest<T> {
    sub_forest: IronedForest<ExactSize<T>>,
    num_trees: usize,
}

impl<T> ExactSizeIronedForest<T> {
    pub fn new() -> ExactSizeIronedForest<T> {
        ExactSizeIronedForest {
            sub_forest: IronedForest::new(),
            num_trees: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> ExactSizeIronedForest<T> {
        ExactSizeIronedForest {
            sub_forest: IronedForest::with_capacity(capacity),
            num_trees: 0,
        }
    }

    pub fn build_tree<R>(
        &mut self,
        initial_root_val: T,
        node_builder_cb: impl FnOnce(ExactSizeNodeBuilder<T>) -> R,
    ) -> R {
        node_builder_cb(self.get_tree_builder(initial_root_val))
    }
    
    pub fn add_single_node_tree(&mut self, val: T) {
        self.get_tree_builder(val);
    }

    pub fn get_tree_builder(&mut self, initial_root_val: T) -> ExactSizeNodeBuilder<T> {
        self.num_trees += 1;

        let exact_size = ExactSize {
            val: initial_root_val,
            num_children: 0,
        };
        ExactSizeNodeBuilder {
            node_builder: self.sub_forest.get_tree_builder(exact_size),
        }
    }

    pub fn iter_trees(&self) -> ExactSizeNodeIter<T> {
        ExactSizeNodeIter {
            iter: self.sub_forest.iter_trees(),
            len: self.num_trees(),
        }
    }

    pub fn iter_trees_mut(&mut self) -> ExactSizeNodeIterMut<T> {
        let len = self.num_trees();
        ExactSizeNodeIterMut {
            iter: self.sub_forest.iter_trees_mut(),
            len,
        }
    }

    pub fn drain_trees(&mut self) -> ExactSizeNodeListDrain<'_, T> {
        let num_trees = self.num_trees();
        ExactSizeNodeListDrain {
            drain: self.sub_forest.drain_trees(),
            len: num_trees,
        }
    }

    pub fn iter_flattened<'a>(
        &'a self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::slice::Iter<'a, NodeData<ExactSize<T>>>,
            impl FnMut(&'a NodeData<ExactSize<T>>) -> &'a ExactSize<T>,
        >,
        impl FnMut(&'a ExactSize<T>) -> &'a T,
    > {
        self.sub_forest
            .iter_flattened()
            .map(|exact_size| &exact_size.val)
    }

    pub fn iter_flattened_mut<'a>(
        &'a mut self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::slice::IterMut<'a, NodeData<ExactSize<T>>>,
            impl FnMut(&'a mut NodeData<ExactSize<T>>) -> &'a mut ExactSize<T>,
        >,
        impl FnMut(&'a mut ExactSize<T>) -> &'a mut T,
    > {
        self.sub_forest
            .iter_flattened_mut()
            .map(|exact_size| &mut exact_size.val)
    }

    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::vec::Drain<NodeData<ExactSize<T>>>,
            impl FnMut(NodeData<ExactSize<T>>) -> ExactSize<T>,
        >,
        impl FnMut(ExactSize<T>) -> T,
    > {
        self.sub_forest
            .drain_flattened()
            .map(|exact_size| exact_size.val)
    }

    pub fn raw(&self) -> &IronedForest<ExactSize<T>> {
        &self.sub_forest
    }

    /// Read-only view of the raw data.
    pub fn raw_data(&self) -> &Vec<NodeData<ExactSize<T>>> {
        self.sub_forest.raw_data()
    }

    pub fn num_trees(&self) -> usize {
        self.num_trees
    }

    pub fn tot_num_nodes(&self) -> usize {
        self.sub_forest.tot_num_nodes()
    }
}

/// test
pub struct ExactSizeNodeBuilder<'a, T> {
    node_builder: NodeBuilder<'a, ExactSize<T>>,
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

    pub fn build_child<R>(
        &mut self,
        initial_val: T,
        child_builder_cb: impl FnOnce(ExactSizeNodeBuilder<T>) -> R,
    ) -> R {
        child_builder_cb(self.get_child_builder(initial_val))
    }

    pub fn add_child(&mut self, val: T) {
        self.get_child_builder(val);
    }

    pub fn get_child_builder(&mut self, initial_val: T) -> ExactSizeNodeBuilder<T> {
        self.node_builder.val_mut().num_children += 1;

        let exact_size = ExactSize {
            val: initial_val,
            num_children: 0,
        };
        ExactSizeNodeBuilder {
            node_builder: self.node_builder.get_child_builder(exact_size),
        }
    }
}

/// test
pub struct ExactSizeNodeIter<'t, T> {
    iter: NodeIter<'t, ExactSize<T>>,
    len: usize,
}

impl<'t, T> Iterator for ExactSizeNodeIter<'t, T> {
    type Item = ExactSizeNodeRef<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(node_ref) => {
                debug_assert!(self.len > 0);
                self.len -= 1;
                Some(ExactSizeNodeRef { node_ref })
            }
            None => {
                debug_assert_eq!(self.len, 0);
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'t, T> ExactSizeIterator for ExactSizeNodeIter<'t, T> {
    fn len(&self) -> usize {
        self.len
    }
}

// TODO: implement TrustedLen for ExactSizeNodeIter when that is stable.

/// test
pub struct ExactSizeNodeRef<'t, T> {
    node_ref: NodeRef<'t, ExactSize<T>>,
}

impl<'t, T> ExactSizeNodeRef<'t, T> {
    pub fn children(self) -> ExactSizeNodeIter<'t, T> {
        let len = self.num_children();
        ExactSizeNodeIter {
            iter: self.node_ref.children(),
            len,
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
    len: usize,
}

impl<'t, T> Iterator for ExactSizeNodeIterMut<'t, T> {
    type Item = ExactSizeNodeRefMut<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(node_ref) => {
                debug_assert!(self.len > 0);
                self.len -= 1;
                Some(ExactSizeNodeRefMut { node_ref })
            }
            None => {
                debug_assert_eq!(self.len, 0);
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'t, T> ExactSizeIterator for ExactSizeNodeIterMut<'t, T> {
    fn len(&self) -> usize {
        self.len
    }
}

/// test
pub struct ExactSizeNodeRefMut<'t, T> {
    node_ref: NodeRefMut<'t, ExactSize<T>>,
}

impl<'t, T> ExactSizeNodeRefMut<'t, T> {
    pub fn into_children(self) -> ExactSizeNodeIterMut<'t, T> {
        let len = self.num_children();
        ExactSizeNodeIterMut {
            iter: self.node_ref.into_children(),
            len,
        }
    }

    pub fn children(&mut self) -> ExactSizeNodeIterMut<T> {
        let len = self.num_children();
        ExactSizeNodeIterMut {
            iter: self.node_ref.children(),
            len,
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

/// test
pub struct ExactSizeNodeListDrain<'t, T> {
    drain: NodeListDrain<'t, ExactSize<T>>,
    len: usize,
}

impl<'t, T> Iterator for ExactSizeNodeListDrain<'t, T> {
    type Item = ExactSizeNodeDrain<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.drain.next() {
            Some(node) => {
                debug_assert!(self.len > 0);
                self.len -= 1;
                Some(ExactSizeNodeDrain { node })
            }
            None => {
                debug_assert_eq!(self.len, 0);
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'t, T> ExactSizeIterator for ExactSizeNodeListDrain<'t, T> {
    fn len(&self) -> usize {
        self.len
    }
}

/// test
pub struct ExactSizeNodeDrain<'t, T> {
    node: NodeDrain<'t, ExactSize<T>>,
}

impl<'t, T> ExactSizeNodeDrain<'t, T> {
    pub fn into_val_and_children(self) -> (T, ExactSizeNodeListDrain<'t, T>) {
        let len = self.num_children();
        let (val, children) = self.node.into_val_and_children();
        (
            val.val,
            ExactSizeNodeListDrain {
                drain: children,
                len,
            },
        )
    }

    pub fn val(&self) -> &T {
        &self.node.val().val
    }

    pub fn num_children(&self) -> usize {
        self.node.val().num_children
    }
}

pub struct ExactSizeIronedTree<T> {
    forest: ExactSizeIronedForest<T>,
}

impl<T> ExactSizeIronedTree<T> {
    pub fn new(
        root_val: T,
        node_builder_cb: impl FnOnce(ExactSizeNodeBuilder<T>),
    ) -> ExactSizeIronedTree<T> {
        ExactSizeIronedTree::new_with_return_val(root_val, node_builder_cb).0
    }

    pub fn new_with_return_val<R>(
        root_val: T,
        node_builder_cb: impl FnOnce(ExactSizeNodeBuilder<T>) -> R,
    ) -> (ExactSizeIronedTree<T>, R) {
        let mut forest = ExactSizeIronedForest::new();
        let ret = forest.build_tree(root_val, node_builder_cb);
        (ExactSizeIronedTree { forest }, ret)
    }

    pub fn new_with_capacity(
        root_val: T,
        node_builder_cb: impl FnOnce(ExactSizeNodeBuilder<T>),
        capacity: usize,
    ) -> ExactSizeIronedTree<T> {
        ExactSizeIronedTree::new_with_capacity_and_return_val(root_val, node_builder_cb, capacity).0
    }

    pub fn new_with_capacity_and_return_val<R>(
        root_val: T,
        node_builder_cb: impl FnOnce(ExactSizeNodeBuilder<T>) -> R,
        capacity: usize,
    ) -> (ExactSizeIronedTree<T>, R) {
        let mut forest = ExactSizeIronedForest::with_capacity(capacity);
        let ret = forest.build_tree(root_val, node_builder_cb);
        (ExactSizeIronedTree { forest }, ret)
    }

    pub fn root(&self) -> ExactSizeNodeRef<T> {
        self.forest.iter_trees().next().unwrap()
    }

    pub fn root_mut(&mut self) -> ExactSizeNodeRefMut<T> {
        self.forest.iter_trees_mut().next().unwrap()
    }

    pub fn drain_root(&mut self) -> ExactSizeNodeDrain<T> {
        self.forest.drain_trees().next().unwrap()
    }

    pub fn iter_flattened<'a>(
        &'a self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::slice::Iter<'a, NodeData<ExactSize<T>>>,
            impl FnMut(&'a NodeData<ExactSize<T>>) -> &'a ExactSize<T>,
        >,
        impl FnMut(&'a ExactSize<T>) -> &'a T,
    > {
        self.forest.iter_flattened()
    }

    pub fn iter_flattened_mut<'a>(
        &'a mut self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::slice::IterMut<'a, NodeData<ExactSize<T>>>,
            impl FnMut(&'a mut NodeData<ExactSize<T>>) -> &'a mut ExactSize<T>,
        >,
        impl FnMut(&'a mut ExactSize<T>) -> &'a mut T,
    > {
        self.forest.iter_flattened_mut()
    }

    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::vec::Drain<NodeData<ExactSize<T>>>,
            impl FnMut(NodeData<ExactSize<T>>) -> ExactSize<T>,
        >,
        impl FnMut(ExactSize<T>) -> T,
    > {
        self.forest.drain_flattened()
    }

    /// Read-only view of the raw data.
    pub fn raw_data(&self) -> &Vec<NodeData<ExactSize<T>>> {
        self.forest.raw_data()
    }

    pub fn tot_num_nodes(&self) -> usize {
        self.forest.tot_num_nodes()
    }
}

impl<T> TryFrom<ExactSizeIronedForest<T>> for ExactSizeIronedTree<T> {
    type Error = ();
    fn try_from(forest: ExactSizeIronedForest<T>) -> Result<Self, Self::Error> {
        if forest.num_trees == 1 {
            Ok(ExactSizeIronedTree { forest })
        } else {
            Err(())
        }
    }
}
*/
