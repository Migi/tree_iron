#![cfg(any(feature = "serde", test))]

use ::serde::{Deserialize, Serialize, Serializer};
use ::serde::ser::{SerializeSeq, SerializeStruct};

use crate::*;

use std::ops::Deref;
use std::clone::Clone;

/*#[derive(Deserialize)]
struct FlatNode<T> {
    val: T,
    next_sibling_offset: usize
}*/

impl<T: Serialize> Serialize for TreeStore<T> {
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

static const RECURSIVE_NODE_STRUCT_NAME: &str = "RecNode";
static const RECURSIVE_NODE_FIELD_VAL: &str = "val";
static const RECURSIVE_NODE_FIELD_CHILDREN: &str = "children";
static const RECURSIVE_NODE_FIELDS: &'static [&'static str] = &[RECURSIVE_NODE_FIELD_VAL, RECURSIVE_NODE_FIELD_CHILDREN];

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum RecursiveNodeField {
    Val,
    Children
}

fn decode_recursive_node_field(field: &str) -> Option<RecursiveNodeField> {
    match value {
        RECURSIVE_NODE_FIELD_VAL => Some(RecursiveNodeField::Val),
        RECURSIVE_NODE_FIELD_CHILDREN => Some(RecursiveNodeField::Children),
        _ => None,
    }
}

impl<'t, T: Serialize> Serialize for NodeRef<'t, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct(RECURSIVE_NODE_STRUCT_NAME, 2)?;
        s.serialize_field(RECURSIVE_NODE_FIELD_VAL, self.val())?;
        s.serialize_field(RECURSIVE_NODE_FIELD_CHILDREN, &self.children())?;
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
        s.serialize_field("next_sibling_offset", &match self.next_sibling_offset() {
            Some(offset) => offset.get(),
            None => 0
        })?;
        s.end()
    }
}

impl<'de, T: Deserialize> Deserialize<'de> for TreeStore<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        if deserializer.is_human_readable() {
            let mut result = TreeStore::new();

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
                    deserializer.deserialize_struct(RECURSIVE_NODE_STRUCT_NAME, RECURSIVE_NODE_FIELDS, self)
                }
            }

            impl<'de, 'a, 'b, T> Visitor<'de> for RecNodeDeserializer<'a, 'b, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "struct RecNode")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let val = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                    self.node_builder.build_child(val, move |node_builder| {
                        seq.next_element_seed(ChildrenDeserializer {
                            node_builder: self.node_builder
                        })?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                    })?;
                    
                    Ok(())
                }

                fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
                where
                    A: MapAccess<'de>,
                {
                    // FIXME: RecNode currently needs to be serialized in order.
                    // I don't see a way around this unfortunately.
                    let val = if let Some(RecursiveNodeField::Val) = map.next_key()? {
                        map.next_value()?
                    } else {
                        return Err(de::Error::unknown_field("unknown", "val"));
                    }
                    
                    if let Some(RecursiveNodeField::Children) = map.next_key()? {
                        self.node_builder.build_child(val, |node_builder| {
                            seq.next_value_seed(ChildrenDeserializer {
                                node_builder
                            })?;
                        })?;
                    } else {
                        return Err(de::Error::unknown_field("unknown", "children"));
                    }

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
                    write!(formatter, "a sequence of RecNode structs")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    while let Some(_) = seq.next_element_seed(RecNodeDeserializer {
                        node_builder: self.node_builder
                    })
                    
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
                    deserializer.deserialize_struct(RECURSIVE_NODE_STRUCT_NAME, RECURSIVE_NODE_FIELDS, self)
                }
            }

            impl<'de, 'a, T> Visitor<'de> for RootNodeDeserializer<'a, T>
            where
                T: Deserialize<'de>,
            {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "struct RecNode")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let val = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                    self.tree_store_mut_ref.build_tree(val, move |node_builder| {
                        seq.next_element_seed(ChildrenDeserializer {
                            node_builder
                        })?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                    })?;
                    
                    Ok(())
                }

                fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
                where
                    A: MapAccess<'de>,
                {
                    // FIXME: RecNode currently needs to be serialized in order.
                    // I don't see a way around this unfortunately.
                    let val = if let Some(RecursiveNodeField::Val) = map.next_key()? {
                        map.next_value()?
                    } else {
                        return Err(de::Error::unknown_field("unknown", "val"));
                    }
                    
                    if let Some(RecursiveNodeField::Children) = map.next_key()? {
                        self.node_builder.build_child(val, move |node_builder| {
                            seq.next_value_seed(ChildrenDeserializer {
                                node_builder: self.node_builder
                            })?;
                        })?;
                    } else {
                        return Err(de::Error::unknown_field("unknown", "children"));
                    }

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
                    write!(formatter, "a sequence of RecNode structs")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    while let Some(_) = seq.next_element_seed(RecNodeDeserializer {
                        tree_store_mut_ref: self.tree_store_mut_ref
                    })
                    
                    Ok(())
                }
            }

            struct TreeStoreVisitor(&mut TreeStore<T>);
            impl<'de> Visitor<'de> for TreeStoreVisitor {
                type Value = TreeStore<T>;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("struct Duration")
                }

                fn visit_seq<V>(self, mut seq: V) -> Result<Duration, V::Error>
                where
                    V: SeqAccess<'de>,
                {
                    while let Some(_el) = seq.next_element_seed::<RootNodeDeserializer>()? {
                    }
                }
            }

            deserializer.visit_seq(TreeStoreVisitor);
        } else {

        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_store() -> TreeStore<i32> {
        let mut store = TreeStore::new();
        store.add_tree(1, |mut node| {
            node.add_child(10, |mut node| {
                node.add_leaf_child(11);
                node.add_leaf_child(12);
                node.add_leaf_child(13);
            });
            node.add_leaf_child(20);
            node.add_child(30, |mut node| {
                node.add_leaf_child(31);
                node.add_leaf_child(32);
                node.add_leaf_child(33);
            });
        });
        store
    }

    #[test]
    fn test_iter() {
        let store = build_store();
        let str = ::serde_json::ser::to_string(&store).unwrap();
        println!("{}", str);
        panic!();
    }
}
