#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg(feature = "serde")]
use crate::*;

#[cfg(feature = "serde")]
impl<T: Serialize> Serialize for Immutree<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
		let iter = self.iter();

		let data = self.data();
		let mut seq = serializer.serialize_seq(Some(data.len()))?;
		for node in data {
			seq.serialize_element(node.val)?;
			seq.serialize_element(node.next_sibling_offset)?;
		}
		seq.end();
    }
}

#[cfg(feature = "serde")]
fn serialize_node_list<T: Serialize, S: Serializer>(iter: ImmutreeNodeIter, serializer: S) -> Result<S::Ok, S::Error> {
	let mut seq = serializer.serialize_seq(Some(data.len()))?;
	for node in iter {
	}
	seq.end();
}

#[cfg(feature = "serde")]
impl<T: Deserialize> Deserialize for Immutree<T> {
	fn deserialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
		self.data().serialize(serializer)
    }
}
