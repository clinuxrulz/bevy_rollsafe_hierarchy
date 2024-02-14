use std::ops::Deref;

use super::{RollSafeId, ROLL_SAFE_ID_PLACE_HOLDER};
use bevy::ecs::{component::Component, world::{FromWorld, World}};

// Holds a reference to the parent entity of this entity.
/// This component should only be present on entities that actually have a parent entity.
///
/// Parent entity must have this entity stored in its [`Children`] component.
/// It is hard to set up parent/child relationships manually,
/// consider using higher level utilities like [`BuildChildren::with_children`].
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
/// [`Children`]: super::children::Children
/// [`BuildChildren::with_children`]: crate::child_builder::BuildChildren::with_children
#[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "reflect", reflect(Component, MapEntities, PartialEq))]
pub struct RollSafeParent(pub RollSafeId);

impl RollSafeParent {
    /// Gets the ID of the parent.
    #[inline(always)]
    pub fn get(&self) -> RollSafeId {
        self.0
    }

    /// Gets the parent ID as a slice of length 1.
    #[inline(always)]
    pub fn as_slice(&self) -> &[RollSafeId] {
        std::slice::from_ref(&self.0)
    }
}

// TODO: We need to impl either FromWorld or Default so Parent can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However Parent should only ever be set with a real user-defined entity.  Its worth looking into
// better ways to handle cases like this.
impl FromWorld for RollSafeParent {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        RollSafeParent(ROLL_SAFE_ID_PLACE_HOLDER)
    }
}

impl Deref for RollSafeParent {
    type Target = RollSafeId;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
