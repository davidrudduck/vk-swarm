# Phase 1: idempotent-delete

A dangling shared_task_id no longer makes a node task undeletable: hive 404 falls through to local deletion, genuine hive failures still abort, success path unchanged.
