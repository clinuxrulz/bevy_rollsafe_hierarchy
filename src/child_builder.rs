use super::{alloc_id, get_or_assign_new_id, id_to_entity, RollSafeChildren, RollSafeId, RollSafeParent};
use bevy::ecs::{
    bundle::Bundle,
    entity::Entity,
    system::{Command, Commands, EntityCommands},
    world::{EntityWorldMut, World},
};
use smallvec::{smallvec, SmallVec};

/// Adds `child` to `parent`'s [`Children`], without checking if it is already present there.
///
/// This might cause unexpected results when removing duplicate children.
fn push_child_unchecked(world: &mut World, parent: Entity, child: Entity) {
    let child_id = get_or_assign_new_id(world, child);
    let mut parent = world.entity_mut(parent);
    if let Some(mut children) = parent.get_mut::<RollSafeChildren>() {
        children.0.push(child_id);
    } else {
        parent.insert(RollSafeChildren(smallvec![child_id]));
    }
}

/// Sets [`Parent`] of the `child` to `new_parent`. Inserts [`Parent`] if `child` doesn't have one.
fn update_parent(world: &mut World, child: Entity, new_parent: Entity) -> Option<Entity> {
    let new_parent_id = get_or_assign_new_id(world, new_parent);
    let mut child = world.entity_mut(child);
    if let Some(mut parent) = child.get_mut::<RollSafeParent>() {
        let previous = parent.0;
        *parent = RollSafeParent(new_parent_id);
        id_to_entity(world, previous)
    } else {
        child.insert(RollSafeParent(new_parent_id));
        None
    }
}

/// Remove child from the parent's [`Children`] component.
///
/// Removes the [`Children`] component from the parent if it's empty.
fn remove_from_children(world: &mut World, parent: Entity, child: Entity) {
    let Some(child_id) = world.get::<RollSafeId>(child).map(|x| *x) else { return; };
    let Some(mut parent) = world.get_entity_mut(parent) else {
        return;
    };
    let Some(mut children) = parent.get_mut::<RollSafeChildren>() else {
        return;
    };
    children.0.retain(|x| *x != child_id);
    if children.is_empty() {
        parent.remove::<RollSafeChildren>();
    }
}

/// Update the [`Parent`] component of the `child`.
/// Removes the `child` from the previous parent's [`Children`].
///
/// Does not update the new parents [`Children`] component.
///
/// Does nothing if `child` was already a child of `parent`.
///
/// Sends [`HierarchyEvent`]'s.
fn update_old_parent(world: &mut World, child: Entity, parent: Entity) {
    let previous = update_parent(world, child, parent);
    if let Some(previous_parent) = previous {
        // Do nothing if the child was already parented to this entity.
        if previous_parent == parent {
            return;
        }
        remove_from_children(world, previous_parent, child);
    }
}

/// Update the [`Parent`] components of the `children`.
/// Removes the `children` from their previous parent's [`Children`].
///
/// Does not update the new parents [`Children`] component.
///
/// Does nothing for a child if it was already a child of `parent`.
///
/// Sends [`HierarchyEvent`]'s.
fn update_old_parents(world: &mut World, parent: Entity, children: &[Entity]) {
    for &child in children {
        if let Some(previous) = update_parent(world, child, parent) {
            // Do nothing if the entity already has the correct parent.
            if parent == previous {
                continue;
            }

            remove_from_children(world, previous, child);
        }
    }
}

/// Removes entities in `children` from `parent`'s [`Children`], removing the component if it ends up empty.
/// Also removes [`Parent`] component from `children`.
fn remove_children(parent: Entity, children: &[Entity], world: &mut World) {
    let mut children2: SmallVec<[RollSafeId; 8]> = SmallVec::new();
    if let Some(parent_children) = world.get::<RollSafeChildren>(parent) {
        for &child in children {
            let Some(child_id) = world.get::<RollSafeId>(child) else { continue; };
            if parent_children.contains(&child_id) {
                children2.push(*child_id);
            }
        }
    } else {
        return;
    }
    for &child in children {
        world.entity_mut(child).remove::<RollSafeParent>();
    }

    let mut parent = world.entity_mut(parent);
    if let Some(mut parent_children) = parent.get_mut::<RollSafeChildren>() {
        parent_children
            .0
            .retain(|parent_child| !children2.contains(parent_child));

        if parent_children.is_empty() {
            parent.remove::<RollSafeChildren>();
        }
    }
}

/// Removes all children from `parent` by removing its [`Children`] component, as well as removing
/// [`Parent`] component from its children.
fn clear_children(parent: Entity, world: &mut World) {
    if let Some(children) = world.entity_mut(parent).take::<RollSafeChildren>() {
        for &child in &children.0 {
            let Some(child) = id_to_entity(world, child) else { continue; };
            world.entity_mut(child).remove::<RollSafeParent>();
        }
    }
}

/// Command that adds a child to an entity.
#[derive(Debug)]
pub struct PushChild {
    /// Parent entity to add the child to.
    pub parent: Entity,
    /// Child entity to add.
    pub child: Entity,
}

impl Command for PushChild {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.parent).add_child(self.child);
    }
}

/// Command that inserts a child at a given index of a parent's children, shifting following children back.
#[derive(Debug)]
pub struct InsertChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
    index: usize,
}

impl Command for InsertChildren {
    fn apply(self, world: &mut World) {
        world
            .entity_mut(self.parent)
            .insert_children(self.index, &self.children);
    }
}

/// Command that pushes children to the end of the entity's [`Children`].
#[derive(Debug)]
pub struct PushChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

impl Command for PushChildren {
    fn apply(self, world: &mut World) {
        for child in &self.children {
            let id = alloc_id(world);
            if let Some(mut id2) = world.get_mut(*child) {
                *id2 = id;
            } else {
                world.entity_mut(*child).insert(id);
            }
        }
        world.entity_mut(self.parent).push_children(&self.children);
    }
}

/// Command that removes children from an entity, and removes these children's parent.
pub struct RemoveChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

impl Command for RemoveChildren {
    fn apply(self, world: &mut World) {
        remove_children(self.parent, &self.children, world);
    }
}

/// Command that clears all children from an entity and removes [`Parent`] component from those
/// children.
pub struct ClearChildren {
    parent: Entity,
}

impl Command for ClearChildren {
    fn apply(self, world: &mut World) {
        clear_children(self.parent, world);
    }
}

/// Command that clear all children from an entity, replacing them with the given children.
pub struct ReplaceChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

impl Command for ReplaceChildren {
    fn apply(self, world: &mut World) {
        clear_children(self.parent, world);
        world.entity_mut(self.parent).push_children(&self.children);
    }
}

/// Command that removes the parent of an entity, and removes that entity from the parent's [`Children`].
pub struct RemoveParent {
    /// `Entity` whose parent must be removed.
    pub child: Entity,
}

impl Command for RemoveParent {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.child).remove_parent();
    }
}

/// Struct for building children entities and adding them to a parent entity.
///
/// # Example
///
/// This example creates three entities, a parent and two children.
///
/// ```
/// # use bevy_ecs::bundle::Bundle;
/// # use bevy_ecs::system::Commands;
/// # use bevy_hierarchy::BuildChildren;
/// # #[derive(Bundle)]
/// # struct MyBundle {}
/// # #[derive(Bundle)]
/// # struct MyChildBundle {}
/// #
/// # fn test(mut commands: Commands) {
/// commands.spawn(MyBundle {}).with_children(|child_builder| {
///     child_builder.spawn(MyChildBundle {});
///     child_builder.spawn(MyChildBundle {});
/// });
/// # }
/// ```
pub struct ChildBuilder<'w, 's, 'a> {
    commands: &'a mut Commands<'w, 's>,
    push_children: PushChildren,
}

impl<'w, 's, 'a> ChildBuilder<'w, 's, 'a> {
    /// Spawns an entity with the given bundle and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn(&'a mut self, bundle: impl Bundle) -> EntityCommands<'w,'s,'a> {
        let e: EntityCommands<'w, 's, 'a> = self.commands.spawn(bundle);
        self.push_children.children.push(e.id());
        e
    }

    /// Spawns an [`Entity`] with no components and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn_empty(&'a mut self) -> EntityCommands<'w,'s,'a> {
        let e = self.commands.spawn_empty();
        self.push_children.children.push(e.id());
        e
    }

    /// Returns the parent entity of this [`ChildBuilder`].
    pub fn parent_entity(&self) -> Entity {
        self.push_children.parent
    }

    /// Adds a command to be executed, like [`Commands::add`].
    pub fn add_command<C: Command>(&mut self, command: C) -> &mut Self {
        self.commands.add(command);
        self
    }
}

/// Trait for removing, adding and replacing children and parents of an entity.
pub trait BuildChildren {
    /// Takes a closure which builds children for this entity using [`ChildBuilder`].
    fn with_children(&mut self, f: impl FnOnce(&mut ChildBuilder)) -> &mut Self;
    /// Pushes children to the back of the builder's children. For any entities that are
    /// already a child of this one, this method does nothing.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Inserts children at the given index.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;
    /// Removes the given children
    ///
    /// Removing all children from a parent causes its [`Children`] component to be removed from the entity.
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Adds a single child.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if the child is the same as the parent.
    fn add_child(&mut self, child: Entity) -> &mut Self;
    /// Removes all children from this entity. The [`Children`] component will be removed if it exists, otherwise this does nothing.
    fn clear_children(&mut self) -> &mut Self;
    /// Removes all current children from this entity, replacing them with the specified list of entities.
    ///
    /// The removed children will have their [`Parent`] component removed.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn replace_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Sets the parent of this entity.
    ///
    /// If this entity already had a parent, the parent's [`Children`] component will have this
    /// child removed from its list. Removing all children from a parent causes its [`Children`]
    /// component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if the parent is the same as the child.
    fn set_parent(&mut self, parent: Entity) -> &mut Self;
    /// Removes the [`Parent`] of this entity.
    ///
    /// Also removes this entity from its parent's [`Children`] component. Removing all children from a parent causes
    /// its [`Children`] component to be removed from the entity.
    fn remove_parent(&mut self) -> &mut Self;
}

impl BuildChildren for EntityCommands<'_,'_,'_> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut ChildBuilder)) -> &mut Self {
        let parent = self.id();
        let mut builder = ChildBuilder {
            commands: self.commands(),
            push_children: PushChildren {
                children: SmallVec::default(),
                parent,
            },
        };

        spawn_children(&mut builder);
        let children = builder.push_children;
        if children.children.contains(&parent) {
            panic!("Entity cannot be a child of itself.");
        }
        self.commands().add(children);
        self
    }

    fn push_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot push entity as a child of itself.");
        }
        self.commands().add(PushChildren {
            children: SmallVec::from(children),
            parent,
        });
        self
    }

    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot insert entity as a child of itself.");
        }
        self.commands().add(InsertChildren {
            children: SmallVec::from(children),
            index,
            parent,
        });
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(RemoveChildren {
            children: SmallVec::from(children),
            parent,
        });
        self
    }

    fn add_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.id();
        if child == parent {
            panic!("Cannot add entity as a child of itself.");
        }
        self.commands().add(PushChild { child, parent });
        self
    }

    fn clear_children(&mut self) -> &mut Self {
        let parent = self.id();
        self.commands().add(ClearChildren { parent });
        self
    }

    fn replace_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot replace entity as a child of itself.");
        }
        self.commands().add(ReplaceChildren {
            children: SmallVec::from(children),
            parent,
        });
        self
    }

    fn set_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        if child == parent {
            panic!("Cannot set parent to itself");
        }
        self.commands().add(PushChild { child, parent });
        self
    }

    fn remove_parent(&mut self) -> &mut Self {
        let child = self.id();
        self.commands().add(RemoveParent { child });
        self
    }
}

/// Struct for adding children to an entity directly through the [`World`] for use in exclusive systems.
#[derive(Debug)]
pub struct WorldChildBuilder<'w> {
    world: &'w mut World,
    parent: Entity,
    parent_id: RollSafeId,
}

impl<'w> WorldChildBuilder<'w> {
    /// Spawns an entity with the given bundle and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn(&mut self, bundle: impl Bundle + Send + Sync + 'static) -> EntityWorldMut<'_> {
        let entity = self.world.spawn((bundle, RollSafeParent(self.parent_id))).id();
        push_child_unchecked(self.world, self.parent, entity);
        self.world.entity_mut(entity)
    }

    /// Spawns an [`Entity`] with no components and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn_empty(&mut self) -> EntityWorldMut<'_> {
        let entity = self.world.spawn(RollSafeParent(self.parent_id)).id();
        push_child_unchecked(self.world, self.parent, entity);
        self.world.entity_mut(entity)
    }

    /// Returns the parent entity of this [`WorldChildBuilder`].
    pub fn parent_entity(&self) -> Entity {
        self.parent
    }
}

/// Trait that defines adding, changing and children and parents of an entity directly through the [`World`].
pub trait BuildWorldChildren {
    /// Takes a closure which builds children for this entity using [`WorldChildBuilder`].
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self;

    /// Adds a single child.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if the child is the same as the parent.
    fn add_child(&mut self, child: Entity) -> &mut Self;

    /// Pushes children to the back of the builder's children. For any entities that are
    /// already a child of this one, this method does nothing.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Inserts children at the given index.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;
    /// Removes the given children
    ///
    /// Removing all children from a parent causes its [`Children`] component to be removed from the entity.
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;

    /// Sets the parent of this entity.
    ///
    /// If this entity already had a parent, the parent's [`Children`] component will have this
    /// child removed from its list. Removing all children from a parent causes its [`Children`]
    /// component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if the parent is the same as the child.
    fn set_parent(&mut self, parent: Entity) -> &mut Self;

    /// Removes the [`Parent`] of this entity.
    ///
    /// Also removes this entity from its parent's [`Children`] component. Removing all children from a parent causes
    /// its [`Children`] component to be removed from the entity.
    fn remove_parent(&mut self) -> &mut Self;
    /// Removes all children from this entity. The [`Children`] component will be removed if it exists, otherwise this does nothing.
    fn clear_children(&mut self) -> &mut Self;
    /// Removes all current children from this entity, replacing them with the specified list of entities.
    ///
    /// The removed children will have their [`Parent`] component removed.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn replace_children(&mut self, children: &[Entity]) -> &mut Self;
}

impl<'w> BuildWorldChildren for EntityWorldMut<'w> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            let parent_id: RollSafeId;
            {
                let parent_id2 = world.get::<RollSafeId>(parent).map(|x| *x);
                if let Some(parent_id3) = parent_id2 {
                    parent_id = parent_id3;
                } else {
                    parent_id = alloc_id(world);
                    world.entity_mut(parent).insert(parent_id);
                }
            }
            spawn_children(&mut WorldChildBuilder { world, parent, parent_id, });
        });
        self
    }

    fn add_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.id();
        if child == parent {
            panic!("Cannot add entity as a child of itself.");
        }
        let child_id = self.world_scope(|world| {
            update_old_parent(world, child, parent);
            return get_or_assign_new_id(world, child);
        });
        if let Some(mut children_component) = self.get_mut::<RollSafeChildren>() {
            children_component.0.retain(|value| child_id != *value);
            children_component.0.push(child_id);
        } else {
            self.insert(RollSafeChildren(smallvec![child_id]));
        }
        self
    }

    fn push_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot push entity as a child of itself.");
        }
        let children2 = self.world_scope(|world| {
            update_old_parents(world, parent, children);
            let mut children2 = SmallVec::<[RollSafeId; 8]>::new();
            for child in children {
                children2.push(get_or_assign_new_id(world, *child));
            }
            return children2;
        });
        if let Some(mut children_component) = self.get_mut::<RollSafeChildren>() {
            children_component
                .0
                .retain(|value| !children2.contains(value));
            children_component.0.extend(children2.iter().cloned());
        } else {
            self.insert(RollSafeChildren(children2));
        }
        self
    }

    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot insert entity as a child of itself.");
        }
        let children2 = self.world_scope(|world| {
            update_old_parents(world, parent, children);
            let mut children2 = SmallVec::<[RollSafeId; 8]>::new();
            for child in children {
                children2.push(get_or_assign_new_id(world, *child));
            }
            return children2;
        });
        if let Some(mut children_component) = self.get_mut::<RollSafeChildren>() {
            children_component
                .0
                .retain(|value| !children2.contains(value));
            children_component.0.insert_from_slice(index, children2.as_slice());
        } else {
            self.insert(RollSafeChildren(children2));
        }
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            remove_children(parent, children, world);
        });
        self
    }

    fn set_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        self.world_scope(|world| {
            world.entity_mut(parent).add_child(child);
        });
        self
    }

    fn remove_parent(&mut self) -> &mut Self {
        let child = self.id();
        if let Some(parent) = self.take::<RollSafeParent>().map(|p| p.get()) {
            self.world_scope(|world| {
                if let Some(parent) = id_to_entity(world, parent) {
                    remove_from_children(world, parent, child);
                }
            });
        }
        self
    }

    fn clear_children(&mut self) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            clear_children(parent, world);
        });
        self
    }

    fn replace_children(&mut self, children: &[Entity]) -> &mut Self {
        self.clear_children().push_children(children)
    }
}
