use serde::{Deserialize, Deserializer};

pub fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        _ => Ok(true),
    }
}

#[macro_export]
macro_rules! vec_to_hashmap {
    ($vec_ref:expr,$($field_name:ident$(.)?)+) => {{
        let vec = $vec_ref;
        let mut ids = std::collections::HashMap::with_capacity(vec.len());
        for item in vec {
            let item_c = item.clone();
            let v = item $(.$field_name)+;
            ids.insert(v.clone(), item_c);
        }
        ids
    }};
}

