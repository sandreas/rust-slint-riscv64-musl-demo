use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration;

pub fn serialize<S>(dur: &Duration, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ms: u64 = dur
        .as_millis()
        .try_into()
        .map_err(serde::ser::Error::custom)?;
    ser.serialize_u64(ms)
}

pub fn deserialize<'de, D>(de: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let ms = u64::deserialize(de)?;
    Ok(Duration::from_millis(ms))
}
