use bevy::{ecs::{entity::Entity, system::{Query, ResMut, Resource}}, utils::HashMap};

use super::RollSafeId;


#[derive(Resource)]
pub struct IdManager {
    next_id: usize,
    unused_ids: Vec<usize>,
    id_to_entity_id: HashMap<usize, Entity>,
}

impl Default for IdManager {
    fn default() -> Self {
        Self {
            next_id: 0,
            unused_ids: Vec::new(),
            id_to_entity_id: HashMap::new(),
        }
    }
}

impl IdManager {
    pub fn alloc_id(&mut self) -> RollSafeId {
        if let Some(id) = self.unused_ids.pop() {
            return RollSafeId(id);
        }
        let id = self.next_id;
        self.next_id += 1;
        return RollSafeId(id);
    }

    pub fn free_id(&mut self, RollSafeId(id): RollSafeId) {
        self.unused_ids.push(id);
    }

    pub fn lookup_entity(&self, id: RollSafeId) -> Option<Entity> {
        self.id_to_entity_id.get(&id.0).map(|x| x.clone())
    }
}

// Call at the start of each update
pub fn update_id_entity_map(
    mut ids: Query<(Entity, &mut RollSafeId)>,
    mut id_manager: ResMut<IdManager>,
) {
    id_manager.id_to_entity_id.clear();
    for (entity, id) in &mut ids {
        id_manager.id_to_entity_id.insert(id.0, entity);
    }
}
