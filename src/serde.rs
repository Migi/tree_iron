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
    subtree_size: usize,
}

impl<T: Serialize> Serialize for PackedForest<T> {
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
            "subtree_size",
            &self.subtree_size().get(),
        )?;
        s.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for PackedForest<T> {
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
                    write!(formatter, "a node")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let val = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                    let mut child_node_builder = self.node_builder.get_child_builder();
                    seq.next_element_seed(ChildrenDeserializer {
                        node_builder: &mut child_node_builder,
                    })?.ok_or_else(|| de::Error::invalid_length(1, &"can't deserialize children"))?;
                    child_node_builder.finish(val);

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
                tree_store_mut_ref: &'a mut PackedForest<T>,
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
                    write!(formatter, "a node")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let val = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                    let mut child_node_builder = self.tree_store_mut_ref.get_tree_builder();
                    seq.next_element_seed(ChildrenDeserializer {
                        node_builder: &mut child_node_builder,
                    })?
                    .ok_or_else(|| de::Error::invalid_length(1, &"can't deserialize children"))?;
                    child_node_builder.finish(val);

                    Ok(())
                }
            }

            struct RootNodeListDeserializer<'a, T> {
                tree_store_mut_ref: &'a mut PackedForest<T>,
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

            let mut result = PackedForest::new();

            deserializer.deserialize_seq(RootNodeListDeserializer {
                tree_store_mut_ref: &mut result,
            })?;

            Ok(result)
        } else {
            struct FlatNodeListDeserializer<'a, T> {
                tree_store_mut_ref: &'a mut PackedForest<T>,
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
                    fn rec_add_n_children<'de, T: Deserialize<'de>, A: SeqAccess<'de>>(
                        seq: &mut A,
                        n: usize,
                        node_builder: &mut NodeBuilder<T>,
                    ) -> Result<(), A::Error> {
                        let mut num_read = 0;
                        while num_read < n {
                            if let Some(node) = seq.next_element::<FlatNode<T>>()? {
                                num_read += 1;
                                let max_num_left_to_read = n - num_read;
                                if node.subtree_size == 0 {
                                    return Err(de::Error::invalid_length(
                                        num_read,
                                        &"subtree_size invalid",
                                    ));
                                }
                                let n_rec = node.subtree_size - 1;
                                if n_rec > max_num_left_to_read {
                                    return Err(de::Error::invalid_length(
                                        num_read,
                                        &"subtree_size invalid",
                                    ));
                                }
                                let mut node_builder_rec = node_builder.get_child_builder();
                                rec_add_n_children(seq, n_rec, &mut node_builder_rec)?;
                                node_builder_rec.finish(node.val);
                                num_read += n_rec;
                            } else {
                                return Err(de::Error::invalid_length(
                                    num_read,
                                    &"offset too large",
                                ));
                            }
                        }
                        Ok(())
                    }

                    while let Some(node) = seq.next_element::<FlatNode<T>>()? {
                        let subtree_size = node.subtree_size;
                        if subtree_size == 0 {
                            return Err(de::Error::invalid_length(
                                0,
                                &"subtree_size invalid",
                            ));
                        }
                        let mut tree_builder = self.tree_store_mut_ref.get_tree_builder();
                        rec_add_n_children(&mut seq, subtree_size-1, &mut tree_builder)?;
                        tree_builder.finish(node.val);
                    }

                    Ok(())
                }
            }

            let mut result = PackedForest::new();

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

    fn build_store() -> PackedForest<i32> {
        let mut store = PackedForest::new();
        store.build_tree(2, |node| {
            node.build_child(10, |node| {
                node.add_child(11);
                node.add_child(12);
                node.add_child(13);
            });
            node.add_child(20);
            node.build_child(30, |node| {
                node.add_child(31);
                node.add_child(32);
                node.add_child(33);
            });
        });
        store.build_tree(3, |node| {
            node.add_child(10);
            node.build_child(20, |node| {
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
        let store2: PackedForest<i32> = ::serde_json::from_str(&str).unwrap();
        let str2 = ::serde_json::ser::to_string(&store2).unwrap();
        assert_eq!(str, str2);
    }

    #[test]
    fn test_bincode() {
        let store = build_store();
        let vec = ::bincode::serialize(&store).unwrap();
        let store2: PackedForest<i32> = ::bincode::deserialize(&vec[..]).unwrap();
        let vec2 = ::bincode::serialize(&store2).unwrap();
        assert_eq!(vec, vec2);
    }
}
