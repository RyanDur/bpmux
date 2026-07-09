use crate::routing::RoutingIdentity;

#[derive(Default, Clone)]
pub struct RoutingState {
    pub identity: RoutingIdentity,
}
