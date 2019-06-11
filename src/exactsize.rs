use crate::*;

use std::iter::{ExactSizeIterator, Iterator};

// pub because it appears in the return type of iter_flattened (etc). But it's not used anywhere other than that.
#[doc(hidden)]
pub struct ExactSize<T> {
    val: T,
    num_children: usize,
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
        initial_val: T,
        node_builder_cb: impl FnOnce(ExactSizeNodeBuilder<T>) -> R,
    ) -> R {
        node_builder_cb(self.add_tree(initial_val))
    }

    pub fn add_tree(&mut self, initial_val: T) -> ExactSizeNodeBuilder<T> {
        self.num_trees += 1;

        let exact_size = ExactSize {
            val: initial_val,
            num_children: 0,
        };
        ExactSizeNodeBuilder {
            node_builder: self.sub_forest.add_tree(exact_size),
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

    pub fn num_trees(&self) -> usize {
        self.num_trees
    }

    pub fn tot_num_nodes(&self) -> usize {
        self.sub_forest.tot_num_nodes()
    }

    pub fn iter_flattened<'a>(&'a self) -> std::iter::Map<std::iter::Map<std::slice::Iter<'a, NodeData<ExactSize<T>>>, impl FnMut(&'a NodeData<ExactSize<T>>) -> &'a ExactSize<T>>, impl FnMut(&'a ExactSize<T>) -> &'a T> {
        self.sub_forest.iter_flattened().map(|exact_size| &exact_size.val)
    }

    pub fn iter_flattened_mut<'a>(&'a mut self) -> std::iter::Map<std::iter::Map<std::slice::IterMut<'a, NodeData<ExactSize<T>>>, impl FnMut(&'a mut NodeData<ExactSize<T>>) -> &'a mut ExactSize<T>>, impl FnMut(&'a mut ExactSize<T>) -> &'a mut T> {
        self.sub_forest.iter_flattened_mut().map(|exact_size| &mut exact_size.val)
    }

    pub fn drain_flattened(&mut self) -> std::iter::Map<std::iter::Map<std::vec::Drain<NodeData<ExactSize<T>>>, impl FnMut(NodeData<ExactSize<T>>) -> ExactSize<T>>, impl FnMut(ExactSize<T>) -> T> {
        self.sub_forest.drain_flattened().map(|exact_size| exact_size.val)
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
        child_builder_cb(self.add_child(initial_val))
    }

    pub fn add_child(&mut self, initial_val: T) -> ExactSizeNodeBuilder<T> {
        self.node_builder.val_mut().num_children += 1;

        let exact_size = ExactSize {
            val: initial_val,
            num_children: 0,
        };
        ExactSizeNodeBuilder {
            node_builder: self.node_builder.add_child(exact_size),
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
