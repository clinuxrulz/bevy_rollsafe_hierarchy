mod components;
pub use components::{RollSafeId, RollSafeChildren, RollSafeParent};

mod id_manager;
pub use id_manager::{IdManager, update_id_entity_map};

mod child_builder;
pub use child_builder::{BuildChildren, BuildWorldChildren};

use bevy::{app::Plugin, ecs::{entity::Entity, system::{Command, EntityCommands}, world::{EntityWorldMut, World}}};

use self::components::ROLL_SAFE_ID_PLACE_HOLDER;

pub struct RollSafeHierarchy;

impl Plugin for RollSafeHierarchy {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .insert_resource(IdManager::default());
    }
}

pub(crate) fn id_to_entity(world: &World, id: RollSafeId) -> Option<Entity> {
    let Some(id_manager) = world.get_resource::<IdManager>() else { return None; };
    return id_manager.lookup_entity(id);
}

pub(crate) fn alloc_id(world: &mut World) -> RollSafeId {
    let Some(mut id_manager) = world.get_resource_mut::<IdManager>() else { return ROLL_SAFE_ID_PLACE_HOLDER; };
    return id_manager.alloc_id();
}

pub(crate) fn free_id(world: &mut World, id: RollSafeId) {
    let Some(mut id_manager) = world.get_resource_mut::<IdManager>() else { return; };
    return id_manager.free_id(id);
}

pub(crate) fn get_or_assign_new_id(world: &mut World, entity: Entity) -> RollSafeId {
    if let Some(id) = world.get::<RollSafeId>(entity) {
        return *id;
    }
    let id = alloc_id(world);
    world.entity_mut(entity).insert(id);
    return id;
}

fn rollsafe_despawn_recursive(world: &mut World, target: Entity) {
    let mut stack = vec![target];
    let mut children2 = Vec::<RollSafeId>::new();
    while let Some(at) = stack.pop() {
        let at_id: RollSafeId;
        {
            let Some(at_id2) = world.get::<RollSafeId>(at) else { continue; };
            at_id = *at_id2;
        }
        {
            let children: Option<&RollSafeChildren> = world.get(at);
            if let Some(children) = children {
                for child in &children.0 {
                    children2.push(*child);
                }
            }
        }
        for child in children2.drain(0..) {
            if let Some(child_entity) = id_to_entity(world, child) {
                stack.push(child_entity);
            }
        }
        let parent: Option<&RollSafeParent> = world.get(at);
        if let Some(parent) = parent {
            let parent_entity = id_to_entity(world, parent.0);
            if let Some(parent_entity) = parent_entity {
                let mut children_empty = false;
                if let Some(mut children3) = world.get_mut::<RollSafeChildren>(parent_entity) {
                    children3.0.retain(|child| *child != at_id);
                    children_empty = children3.0.is_empty();
                }
                if children_empty {
                    world.entity_mut(parent_entity).remove::<RollSafeChildren>();
                }
            }
        }
        if let Some(mut children) = world.get_mut::<RollSafeChildren>(at) {
            children.0.clear();
        }
        world.despawn(at);
        free_id(world, at_id);
    }
}

struct RollSafeDespawnRecursive {
    target: Entity
}

impl Command for RollSafeDespawnRecursive {
    fn apply(self, world: &mut bevy::prelude::World) {
        rollsafe_despawn_recursive(world, self.target);
    }
}

pub trait RollSafeDespawnRecursiveExt {
    fn rollsafe_despawn_recursive(self);
}

impl<'w> RollSafeDespawnRecursiveExt for EntityWorldMut<'w> {
    fn rollsafe_despawn_recursive(self) {
        let target = self.id();
        rollsafe_despawn_recursive(self.into_world_mut(), target);
    }
}

impl<'w, 's, 'a> RollSafeDespawnRecursiveExt for EntityCommands<'w, 's, 'a> {
    fn rollsafe_despawn_recursive(mut self) {
        let target = self.id();
        self.commands().add(RollSafeDespawnRecursive { target, });
    }
}
