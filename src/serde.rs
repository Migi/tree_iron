#![cfg(any(feature = "serde", test))]

use ::serde::{Deserialize, Deserializer, Serialize, Serializer};
use ::serde::ser::{SerializeSeq, SerializeStruct};
use ::serde::de;
use ::serde::de::{SeqAccess, DeserializeSeed, Visitor};

use crate::*;

use std::ops::Deref;
use std::clone::Clone;
use std::fmt;

#[derive(Deserialize)]
struct FlatNode<T> {
    val: T,
    offset: usize
}

impl<T: Serialize> Serialize for TreeStore<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if !serializer.is_human_readable() {
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
        s.serialize_field("offset", &match self.next_sibling_offset() {
            Some(offset) => offset.get(),
            None => 0
        })?;
        s.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for TreeStore<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        if !deserializer.is_human_readable() {
            struct RecNodeDeserializer<'a, 'b : 'a, T>{
                node_builder: &'a mut NodeBuilder<'b, T>
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
                    let val = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                    let mut child_node_builder = self.node_builder.add_child(val);
                    seq.next_element_seed(ChildrenDeserializer {
                        node_builder: &mut child_node_builder
                    })?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                    
                    Ok(())
                }
            }

            struct ChildrenDeserializer<'a, 'b : 'a, T>{
                node_builder: &'a mut NodeBuilder<'b, T>
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
                        node_builder: self.node_builder
                    })? {}
                    
                    Ok(())
                }
            }

            struct RootNodeDeserializer<'a, T: 'a>{
                tree_store_mut_ref: &'a mut TreeStore<T>
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
                    let val = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                    
                    let mut child_node_builder = self.tree_store_mut_ref.add_tree(val);
                    seq.next_element_seed(ChildrenDeserializer {
                        node_builder: &mut child_node_builder
                    })?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                    
                    Ok(())
                }
            }

            struct RootNodeListDeserializer<'a, T>{
                tree_store_mut_ref: &'a mut TreeStore<T>
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
                        tree_store_mut_ref: self.tree_store_mut_ref
                    })? {}
                    
                    Ok(())
                }
            }
            
            let mut result = TreeStore::new();

            deserializer.deserialize_seq(RootNodeListDeserializer {
                tree_store_mut_ref: &mut result
            })?;

            Ok(result)
        }
        else
        {
            struct FlatNodeListDeserializer<'a, T>{
                tree_store_mut_ref: &'a mut TreeStore<T>
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
                    // returns number of elements read from the list
                    fn rec_add_children_until_offset<'a,'de,T: Deserialize<'de>,A:SeqAccess<'de>>(seq: &mut A, offset: usize, mut node_builder: NodeBuilder<'a,T>) -> Result<usize, A::Error> {
                        let mut num_read = 0;
                        while let Some(node) = seq.next_element::<FlatNode<T>>()? {
                            let node_builder_rec = node_builder.add_child(node.val);
                            let num_read_rec = rec_add_children_until_offset(seq, node.offset, node_builder_rec)?;
                            num_read += num_read_rec;
                            if offset == 0 || num_read == offset {
                                break;
                            } else if num_read > offset {
                                return Err(de::Error::invalid_length(num_read, &"wrong offset"));
                            }
                        }
                        Ok(num_read)
                    }

                    while let Some(node) = seq.next_element::<FlatNode<T>>()? {
                        let offset = node.offset;
                        let tree_builder = self.tree_store_mut_ref.add_tree(node.val);
                        rec_add_children_until_offset(&mut seq, offset, tree_builder)?;
                    }
                    
                    Ok(())
                }
            }

            let mut result = TreeStore::new();

            deserializer.deserialize_seq(FlatNodeListDeserializer {
                tree_store_mut_ref: &mut result
            })?;

            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_store() -> TreeStore<i32> {
        let mut store = TreeStore::new();
        store.build_tree(1, |mut node| {
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
        store
    }

    #[test]
    fn test_iter() {
        let store = build_store();
        let str = ::serde_json::ser::to_string(&store).unwrap();
        println!("{}", str);
        let store2 : TreeStore<i32> = ::serde_json::from_str(&str).unwrap();
        let str2 = ::serde_json::ser::to_string(&store2).unwrap();
        assert_eq!(str, str2);
    }
}
