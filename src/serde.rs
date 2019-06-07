#![cfg(any(feature = "serde", test))]

use ::serde::de;
use ::serde::de::{DeserializeSeed, SeqAccess, Visitor};
use ::serde::ser::{SerializeSeq, SerializeStruct};
use ::serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::*;

use std::clone::Clone;
use std::fmt;
use std::ops::Deref;

#[derive(Deserialize)]
struct FlatNode<T> {
    val: T,
    offset: usize,
}

impl<T: Serialize> Serialize for IronedForest<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let mut seq = serializer.serialize_seq(None)?;
            for node in self.iter_trees() {
                seq.serialize_element(&node)?;
            }
            seq.end()
        } else {
            let data = self.raw_data();

            let mut seq = serializer.serialize_seq(Some(data.len()))?;
            for node in data {
                seq.serialize_element(node.deref())?;
            }
            seq.end()
        }
    }
}

impl<'t, T: Serialize> Serialize for NodeIter<'t, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        for node in (*self).clone() {
            seq.serialize_element(&node)?;
        }
        seq.end()
    }
}

impl<'t, T: Serialize> Serialize for NodeRef<'t, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_seq(Some(2))?;
        s.serialize_element(self.val())?;
        s.serialize_element(&self.children())?;
        s.end()
    }
}

impl<T: Serialize> Serialize for NodeData<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("FlatNode", 2)?;
        s.serialize_field("val", self.val())?;
        s.serialize_field(
            "offset",
            &match self.next_sibling_offset() {
                Some(offset) => offset.get(),
                None => 0,
            },
        )?;
        s.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for IronedForest<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            struct RecNodeDeserializer<'a, 'b: 'a, T> {
                node_builder: &'a mut NodeBuilder<'b, T>,
            }

            impl<'de, 'a, 'b, T> DeserializeSeed<'de> for RecNodeDeserializer<'a, 'b, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_seq(self)
                }
            }

            impl<'de, 'a, 'b, T> Visitor<'de> for RecNodeDeserializer<'a, 'b, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a node (which is a sequence of 2 elements)")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let val = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                    let mut child_node_builder = self.node_builder.add_child(val);
                    seq.next_element_seed(ChildrenDeserializer {
                        node_builder: &mut child_node_builder,
                    })?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                    Ok(())
                }
            }

            struct ChildrenDeserializer<'a, 'b: 'a, T> {
                node_builder: &'a mut NodeBuilder<'b, T>,
            }

            impl<'de, 'a, 'b, T> DeserializeSeed<'de> for ChildrenDeserializer<'a, 'b, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_seq(self)
                }
            }

            impl<'de, 'a, 'b, T> Visitor<'de> for ChildrenDeserializer<'a, 'b, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a sequence")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    while let Some(_) = seq.next_element_seed(RecNodeDeserializer {
                        node_builder: self.node_builder,
                    })? {}

                    Ok(())
                }
            }

            struct RootNodeDeserializer<'a, T: 'a> {
                tree_store_mut_ref: &'a mut IronedForest<T>,
            }

            impl<'de, 'a, T> DeserializeSeed<'de> for RootNodeDeserializer<'a, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_seq(self)
                }
            }

            impl<'de, 'a, T> Visitor<'de> for RootNodeDeserializer<'a, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a node (which is a sequence of 2 elements)")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let val = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                    let mut child_node_builder = self.tree_store_mut_ref.add_tree(val);
                    seq.next_element_seed(ChildrenDeserializer {
                        node_builder: &mut child_node_builder,
                    })?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                    Ok(())
                }
            }

            struct RootNodeListDeserializer<'a, T> {
                tree_store_mut_ref: &'a mut IronedForest<T>,
            }

            impl<'de, 'a, T> DeserializeSeed<'de> for RootNodeListDeserializer<'a, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_seq(self)
                }
            }

            impl<'de, 'a, T> Visitor<'de> for RootNodeListDeserializer<'a, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a sequence")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    while let Some(_) = seq.next_element_seed(RootNodeDeserializer {
                        tree_store_mut_ref: self.tree_store_mut_ref,
                    })? {}

                    Ok(())
                }
            }

            let mut result = IronedForest::new();

            deserializer.deserialize_seq(RootNodeListDeserializer {
                tree_store_mut_ref: &mut result,
            })?;

            Ok(result)
        } else {
            struct FlatNodeListDeserializer<'a, T> {
                tree_store_mut_ref: &'a mut IronedForest<T>,
            }

            impl<'de, 'a, T> DeserializeSeed<'de> for FlatNodeListDeserializer<'a, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_seq(self)
                }
            }

            impl<'de, 'a, T> Visitor<'de> for FlatNodeListDeserializer<'a, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a sequence")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    // reads n elements from the SeqAccess and adds them as nodes to the node_builder
                    // if n is None, reads all elements until the end of the stream
                    fn rec_add_n_children<'a, 'de, T: Deserialize<'de>, A: SeqAccess<'de>>(
                        seq: &mut A,
                        n: Option<usize>,
                        mut node_builder: NodeBuilder<'a, T>,
                    ) -> Result<(), A::Error> {
                        match n {
                            Some(n) => {
                                let mut num_read = 0;
                                while num_read < n {
                                    if let Some(node) = seq.next_element::<FlatNode<T>>()? {
                                        num_read += 1;
                                        let max_num_left_to_read = n - num_read;
                                        let n_rec = {
                                            if node.offset == 0 {
                                                max_num_left_to_read
                                            } else {
                                                if node.offset - 1 > max_num_left_to_read {
                                                    return Err(de::Error::invalid_length(
                                                        num_read,
                                                        &"offset invalid",
                                                    ));
                                                }
                                                node.offset - 1
                                            }
                                        };
                                        let node_builder_rec = node_builder.add_child(node.val);
                                        rec_add_n_children(seq, Some(n_rec), node_builder_rec)?;
                                        num_read += n_rec;
                                    } else {
                                        return Err(de::Error::invalid_length(
                                            num_read,
                                            &"offset too large",
                                        ));
                                    }
                                }
                            }
                            None => {
                                while let Some(node) = seq.next_element::<FlatNode<T>>()? {
                                    let n_rec = {
                                        if node.offset == 0 {
                                            None
                                        } else {
                                            Some(node.offset - 1)
                                        }
                                    };
                                    let node_builder_rec = node_builder.add_child(node.val);
                                    rec_add_n_children(seq, n_rec, node_builder_rec)?;
                                }
                            }
                        }
                        Ok(())
                    }

                    while let Some(node) = seq.next_element::<FlatNode<T>>()? {
                        let offset = node.offset;
                        let tree_builder = self.tree_store_mut_ref.add_tree(node.val);
                        if offset == 0 {
                            rec_add_n_children(&mut seq, None, tree_builder)?;
                        } else {
                            rec_add_n_children(&mut seq, Some(offset - 1), tree_builder)?;
                        }
                    }

                    Ok(())
                }
            }

            let mut result = IronedForest::new();

            deserializer.deserialize_seq(FlatNodeListDeserializer {
                tree_store_mut_ref: &mut result,
            })?;

            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_store() -> IronedForest<i32> {
        let mut store = IronedForest::new();
        store.build_tree(2, |mut node| {
            node.build_child(10, |mut node| {
                node.add_child(11);
                node.add_child(12);
                node.add_child(13);
            });
            node.add_child(20);
            node.build_child(30, |mut node| {
                node.add_child(31);
                node.add_child(32);
                node.add_child(33);
            });
        });
        store.build_tree(3, |mut node| {
            node.add_child(10);
            node.build_child(20, |mut node| {
                node.add_child(21);
                node.add_child(22);
                node.add_child(23);
            });
            node.add_child(30);
        });
        store
    }

    #[test]
    fn test_json() {
        let store = build_store();
        let str = ::serde_json::ser::to_string(&store).unwrap();
        let store2: IronedForest<i32> = ::serde_json::from_str(&str).unwrap();
        let str2 = ::serde_json::ser::to_string(&store2).unwrap();
        assert_eq!(str, str2);
    }

    #[test]
    fn test_bincode() {
        let store = build_store();
        let vec = ::bincode::serialize(&store).unwrap();
        let store2: IronedForest<i32> = ::bincode::deserialize(&vec[..]).unwrap();
        let vec2 = ::bincode::serialize(&store2).unwrap();
        assert_eq!(vec, vec2);
    }
}
