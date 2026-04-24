use crate::state::AppState;
use pebble_core::{now_timestamp, KanbanCard, KanbanColumn, PebbleError};
use tauri::State;

#[tauri::command]
pub async fn move_to_kanban(
    state: State<'_, AppState>,
    message_id: String,
    column: KanbanColumn,
    position: Option<i32>,
) -> std::result::Result<(), PebbleError> {
    let now = now_timestamp();
    let card = KanbanCard {
        message_id,
        column,
        position: position.unwrap_or(0),
        created_at: now,
        updated_at: now,
    };
    state.store.upsert_kanban_card(&card)
}

#[tauri::command]
pub async fn list_kanban_cards(
    state: State<'_, AppState>,
    column: Option<KanbanColumn>,
) -> std::result::Result<Vec<KanbanCard>, PebbleError> {
    state.store.list_kanban_cards(column.as_ref())
}

#[tauri::command]
pub async fn remove_from_kanban(
    state: State<'_, AppState>,
    message_id: String,
) -> std::result::Result<(), PebbleError> {
    state.store.delete_kanban_card(&message_id)
}
