/// Core contains all the unsafe code.
/// It should be kept as small as possible.
use std::iter::{ExactSizeIterator, Iterator};
use std::mem::ManuallyDrop;

pub struct Immutree<T> {
    data: Vec<ManuallyDrop<NodeData<T>>>, // all data is dropped in drop(), only drain() prevents this
    num_root_nodes: usize,
}

impl<T> Drop for Immutree<T> {
    fn drop(&mut self) {
        for node in self.data.iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
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

    pub fn iter(&self) -> ImmutreeNodeIter<T> {
        ImmutreeNodeIter {
            tree: self,
            cur_node_index: 0,
            len: self.num_root_nodes
        }
    }

    pub fn iter_mut(&mut self) -> ImmutreeNodeIterMut<T> {
        ImmutreeNodeIterMut {
            remaining_nodes: &mut self.data[..]
        }
    }

    pub fn drain(mut self) -> ImmutreeDrain<T> {
        let data = std::mem::replace(&mut self.data, Vec::new());
        ImmutreeDrain { data, drop_from: 0 }
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
        self.tree.data.push(ManuallyDrop::new(NodeData {
            val,
            num_children: 0,
            num_descendants: 0,
        }));
        let (child_num_children, child_num_descendants, ret) = {
            let mut child_node_builder = ImmutreeNodeBuilder {
                tree: self.tree,
                num_children: 0,
                num_descendants: 0,
            };
            let ret = child_builder_cb(&mut child_node_builder);
            (
                child_node_builder.num_children,
                child_node_builder.num_descendants,
                ret,
            )
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
    len: usize, // num remaining nodes in this iterator
}

pub struct ImmutreeNodeRef<'t, T> {
    tree: &'t Immutree<T>,
    index: usize,
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
                index: result_node_index,
            })
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'t, T> ExactSizeIterator for ImmutreeNodeIter<'t, T> {}

impl<'t, T> ImmutreeNodeRef<'t, T> {
    pub fn children(&self) -> ImmutreeNodeIter<'t, T> {
        unsafe {
            let node = self.tree.data.get_unchecked(self.index);
            ImmutreeNodeIter {
                tree: self.tree,
                cur_node_index: self.index + 1,
                len: node.num_children,
            }
        }
    }

    pub fn val(&self) -> &'t T {
        unsafe { &self.tree.data.get_unchecked(self.index).val }
    }
}

pub struct ImmutreeNodeIterMut<'t, T> {
    remaining_nodes: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the nodes in the iterator and all their descendants
}

pub struct ImmutreeNodeRefMut<'t, T> {
    slice: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the current node and all its descendants
}

impl<'t, T> Iterator for ImmutreeNodeIterMut<'t, T> {
    type Item = ImmutreeNodeRefMut<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes
            .get(0)
            .map(|cur_node| cur_node.num_descendants)
            .map(|num_descendants| {
                let next_node = 1 + num_descendants;
                let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at_mut(next_node);
                self.remaining_nodes = next_nodes_slice;
                ImmutreeNodeRefMut {
                    slice: cur_node_slice,
                }
            })
    }
}

impl<'t, T> ImmutreeNodeRefMut<'t, T> {
    pub fn children(self) -> ImmutreeNodeIterMut<'t, T> {
        let (_, remaining_nodes) = self.slice.split_first_mut().unwrap();
        ImmutreeNodeIterMut { remaining_nodes }
    }

    pub fn val(&self) -> &T {
        unsafe { &self.slice.get_unchecked(0).val }
    }

    pub fn val_mut(&mut self) -> &mut T {
        unsafe { &mut self.slice.get_unchecked_mut(0).val }
    }
}

pub struct ImmutreeDrain<T> {
    data: Vec<ManuallyDrop<NodeData<T>>>, // only data from `drop_from` to the end of the vec is dropped
    drop_from: usize,
}

pub struct ImmutreeNodeListDrain<'t, T> {
    remaining_nodes: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the nodes in the iterator and all their descendants. Drops them in drop().
}

pub struct ImmutreeSingleNodeDrain<'t, T> {
    slice: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the current node and all its descendants. Drops them in drop().
}

impl<T> Drop for ImmutreeDrain<T> {
    fn drop(&mut self) {
        for node in self.data[self.drop_from..].iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<'t, T> Drop for ImmutreeNodeListDrain<'t, T> {
    fn drop(&mut self) {
        for node in self.remaining_nodes.iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<'t, T> Drop for ImmutreeSingleNodeDrain<'t, T> {
    fn drop(&mut self) {
        for node in self.slice.iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<T> ImmutreeDrain<T> {
    pub fn drain_all(&mut self) -> ImmutreeNodeListDrain<T> {
        let drop_from = self.drop_from;
        self.drop_from = self.data.len();
        ImmutreeNodeListDrain {
            remaining_nodes: &mut self.data[drop_from..],
        }
    }
}

impl<'t, T> Iterator for ImmutreeNodeListDrain<'t, T> {
    type Item = ImmutreeSingleNodeDrain<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes
            .get(0)
            .map(|cur_node| cur_node.num_descendants)
            .map(|num_descendants| {
                let next_node = 1 + num_descendants;
                let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at_mut(next_node);
                self.remaining_nodes = next_nodes_slice;
                ImmutreeSingleNodeDrain {
                    slice: cur_node_slice,
                }
            })
    }
}

impl<'t, T> ImmutreeSingleNodeDrain<'t, T> {
    pub fn into_val_and_children(mut self) -> (T, ImmutreeNodeListDrain<'t, T>) {
        let slice = std::mem::replace(&mut self.slice, &mut []);
        let (node_data_ref, remaining_nodes) = slice.split_first_mut().unwrap();
        unsafe {
            let node_data: NodeData<T> = std::ptr::read(&**node_data_ref); // TODO: replace with ManuallyDrop::take() once that is stabilized
            (node_data.val, ImmutreeNodeListDrain { remaining_nodes })
        }
    }

    pub fn val(&self) -> &T {
        unsafe { &self.slice.get_unchecked(0).val }
    }

    pub fn val_mut(&mut self) -> &mut T {
        unsafe { &mut self.slice.get_unchecked_mut(0).val }
    }
}
