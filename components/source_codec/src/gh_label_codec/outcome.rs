use super::GhLabelCodec;
use product_management::story::Horizon;

impl GhLabelCodec {
    pub fn decode_parent_outcome(&self, label: &str) -> Option<(u64, u32)> {
        let rest = label.strip_prefix("parent-outcome-#")?;
        let (number, position) = rest.split_once('.')?;
        Some((number.parse().ok()?, position.parse().ok()?))
    }

    pub fn decode_horizon(&self, label: &str) -> Option<Horizon> {
        match label.strip_prefix("horizon.")? {
            "now" => Some(Horizon::Now),
            "next" => Some(Horizon::Next),
            "later" => Some(Horizon::Later),
            _ => None,
        }
    }

    pub fn encode_horizon(&self, horizon: Horizon) -> String {
        format!("horizon.{}", horizon.as_str())
    }

    pub fn encode_horizon_order(&self, position: u32) -> String {
        format!("horizon-order.{position}")
    }

    pub fn decode_horizon_order(&self, label: &str) -> Option<u32> {
        label.strip_prefix("horizon-order.")?.parse().ok()
    }
}
