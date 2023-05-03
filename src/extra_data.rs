use hcs_lib::data;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ExtraData {}

impl data::Data for ExtraData {}
