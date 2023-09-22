use crate::db_connectors::sqlite::models::Conversation;

pub fn conversation_to_json(conversation: Conversation) -> serde_json::Value {
    serde_json::json!({
        "client": {
            "bot_id": conversation.bot_id,
            "channel_id": conversation.channel_id,
            "user_id": conversation.user_id
        },
        "flow_id": conversation.flow_id,
        "step_id": conversation.step_id,
        "status": conversation.status,
        "last_interaction_at": conversation.last_interaction_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        "updated_at": conversation.updated_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        "created_at": conversation.created_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string()
    })
}
