# Bevy Rollsafe Hierarchy
Proof of concept (untest) hierarchy plugin for bevy can handle rollbacks.
Internally ID numbers are generated for the parent/children rather than using the Entity IDs, so when a rollback occurs the Parent/Children component IDs will still be valid.

To use it, add the ```RollSafeHierarchy``` to your App, and make sure to the execute system ```update_id_entity_map``` repeatedly before anything in ```Update```.
```update_id_entity_map``` updates a resource that maps the ```RollSafeId```s to ```Entity``` IDs.
