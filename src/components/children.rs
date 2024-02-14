use std::{ops::Deref, slice};

use super::RollSafeId;
use bevy::ecs::{component::Component, world::{FromWorld, World}};
use smallvec::SmallVec;

/// Contains references to the child entities of this entity.
///
/// Each child must contain a [`Parent`] component that points back to this entity.
/// This component rarely needs to be created manually,
/// consider using higher level utilities like [`BuildChildren::with_children`]
/// which are safer and easier to use.
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
/// [`Parent`]: crate::components::parent::Parent
/// [`BuildChildren::with_children`]: crate::child_builder::BuildChildren::with_children
#[derive(Component, Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "reflect", reflect(Component, MapEntities))]
pub struct RollSafeChildren(pub(crate) SmallVec<[RollSafeId; 8]>);

// TODO: We need to impl either FromWorld or Default so Children can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However Children should only ever be set with a real user-defined entities. Its worth looking
// into better ways to handle cases like this.
impl FromWorld for RollSafeChildren {
    #[inline]
    fn from_world(_world: &mut World) -> Self {
        RollSafeChildren(SmallVec::new())
    }
}

impl Deref for RollSafeChildren {
    type Target = [RollSafeId];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl<'a> IntoIterator for &'a RollSafeChildren {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = slice::Iter<'a, RollSafeId>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
