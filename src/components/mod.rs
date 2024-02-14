mod children;
mod parent;

pub use children::RollSafeChildren;
pub use parent::RollSafeParent;

use bevy::ecs::component::Component;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RollSafeId(pub(crate) usize);

pub(crate) const ROLL_SAFE_ID_PLACE_HOLDER: RollSafeId = RollSafeId(usize::MAX);
