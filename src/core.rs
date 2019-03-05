/// Core contains all the unsafe code.
/// It should be kept as small as possible.

use std::iter::Iterator;
use std::vec::Drain;

pub struct Immutree<T> {
    data: Vec<NodeData<T>>,
    num_root_nodes: usize,
}

impl<T> Immutree<T> {
    pub fn new() -> Immutree<T> {
        Immutree {
            data: Vec::new(),
            num_root_nodes: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Immutree<T> {
        Immutree {
            data: Vec::with_capacity(capacity),
            num_root_nodes: 0,
        }
    }

    pub fn build_root_node<R>(
        &mut self,
        val: T,
        child_builder_cb: impl FnOnce(&mut ImmutreeNodeBuilder<T>) -> R,
    ) -> R {
        self.num_root_nodes += 1;
        let mut node_builder = ImmutreeNodeBuilder {
            tree: self,
            num_children: 0,
            num_descendants: 0,
        };
        node_builder.build_child(val, child_builder_cb)
    }
}

pub struct NodeData<T> {
    val: T,
    num_children: usize,
    num_descendants: usize,
}

pub struct ImmutreeNodeBuilder<'a, T> {
    tree: &'a mut Immutree<T>,
    num_children: usize,
    num_descendants: usize,
}

impl<'a, T> ImmutreeNodeBuilder<'a, T> {
    pub fn add_leaf_child(&mut self, val: T) {
        self.build_child(val, |_| {});
    }

    pub fn build_child<R>(
        &mut self,
        val: T,
        child_builder_cb: impl FnOnce(&mut ImmutreeNodeBuilder<T>) -> R,
    ) -> R {
        let child_node_index = self.tree.data.len();
        self.tree.data.push(NodeData {
            val,
            num_children: 0,
            num_descendants: 0,
        });
        let (child_num_children, child_num_descendants, ret) = {
            let mut child_node_builder = ImmutreeNodeBuilder {
                tree: self.tree,
                num_children: 0,
                num_descendants: 0,
            };
            let ret = child_builder_cb(&mut child_node_builder);
            (child_node_builder.num_children, child_node_builder.num_descendants, ret)
        };
        self.num_children += 1;
        self.num_descendants += 1 + child_num_descendants;
        unsafe {
            let child_node = self.tree.data.get_unchecked_mut(child_node_index);
            child_node.num_children = child_num_children;
            child_node.num_descendants = child_num_descendants;
        }
        ret
    }
}

pub struct ImmutreeNodeIter<'t, T> {
    tree: &'t Immutree<T>,
    cur_node_index: usize,
    len: usize // num remaining nodes in this iterator
}

pub struct ImmutreeNodeRef<'t, T> {
    tree: &'t Immutree<T>,
    index: usize
}

impl<'t, T> Iterator for ImmutreeNodeIter<'t, T> {
    type Item = ImmutreeNodeRef<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            None
        } else {
            let result_node_index = self.cur_node_index;
            // update cur_node_index
            unsafe {
                let cur_node = self.tree.data.get_unchecked(result_node_index);
                self.cur_node_index += 1 + cur_node.num_descendants;
                self.len -= 1;
            }
            Some(ImmutreeNodeRef {
                tree: self.tree,
                index: result_node_index
            })
        }
    }
}

impl<'t, T> ImmutreeNodeRef<'t, T> {
    pub fn children(&self) -> ImmutreeNodeIter<'t, T> {
        unsafe {
            let node = self.tree.data.get_unchecked(self.index);
            ImmutreeNodeIter {
                tree: self.tree,
                cur_node_index: self.index + 1,
                len: node.num_children
            }
        }
    }

    pub fn val(&self) -> &'t T {
        unsafe {
            &self.tree.data.get_unchecked(self.index).val
        }
    }
}

pub struct ImmutreeNodeIterMut<'t, T> {
    remaining_nodes: &'t mut [NodeData<T>], // contains (only) the nodes in the iterator and all their descendants
}

pub struct ImmutreeNodeRefMut<'t, T> {
    slice: &'t mut [NodeData<T>], // contains (only) the current node and all its descendants
}

impl<'t, T> Iterator for ImmutreeNodeIterMut<'t, T> {
    type Item = ImmutreeNodeRefMut<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes.get(0).map(
            |cur_node| cur_node.num_descendants
        ).map(
            |num_descendants| {
                let next_node = 1 + num_descendants;
                let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []); // I'm not sure why this mem::replace is necessary but it seems to be
                let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at_mut(next_node);
                self.remaining_nodes = next_nodes_slice;
                ImmutreeNodeRefMut {
                    slice: cur_node_slice
                }
            }
        )
    }
}

impl<'t, T> ImmutreeNodeRefMut<'t, T> {
    pub fn children(self) -> ImmutreeNodeIterMut<'t, T> {
        let (_, remaining_nodes) = self.slice.split_first_mut().unwrap();
        ImmutreeNodeIterMut {
            remaining_nodes
        }
    }

    pub fn val(&self) -> &T {
        unsafe {
            &self.slice.get_unchecked(0).val
        }
    }

    pub fn val_mut(&mut self) -> &mut T {
        unsafe {
            &mut self.slice.get_unchecked_mut(0).val
        }
    }
}

pub struct ImmutreeNodeDrain<'t, T> {
    drain: Drain<'t, NodeData<T>>, // contains (only) the nodes in the iterator and all their descendants
}

pub struct ImmutreeNodeDrain2<'t, T> {
    slice: &'t mut [NodeData<T>], // contains (only) the current node and all its descendants
}

impl<'t, T> Iterator for ImmutreeNodeDrain<'t, T> {
    type Item = ImmutreeNodeRefMut<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes.get(0).map(
            |cur_node| cur_node.num_descendants
        ).map(
            |num_descendants| {
                let next_node = 1 + num_descendants;
                let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []); // I'm not sure why this mem::replace is necessary but it seems to be
                let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at_mut(next_node);
                self.remaining_nodes = next_nodes_slice;
                ImmutreeNodeRefMut {
                    slice: cur_node_slice
                }
            }
        )
    }
}

impl<'t, T> ImmutreeNodeRefMut<'t, T> {
    pub fn children(self) -> ImmutreeNodeIterMut<'t, T> {
        let (_, remaining_nodes) = self.slice.split_first_mut().unwrap();
        ImmutreeNodeIterMut {
            remaining_nodes
        }
    }

    pub fn val(&self) -> &T {
        unsafe {
            &self.slice.get_unchecked(0).val
        }
    }

    pub fn val_mut(&mut self) -> &mut T {
        unsafe {
            &mut self.slice.get_unchecked_mut(0).val
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
