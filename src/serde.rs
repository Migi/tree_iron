#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use crate::*;

#[cfg(feature = "serde")]
impl<T: Serialize> Serialize for Immutree<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
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
impl<T: Deserialize> Deserialize for Immutree<T> {
	fn deserialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
		self.data().serialize(serializer)
    }
}
