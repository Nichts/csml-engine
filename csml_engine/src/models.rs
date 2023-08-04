use csml_interpreter::data::{Client, CsmlBot};
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DbConversation {
    pub id: String,
    pub client: Client,
    pub flow_id: String,
    pub step_id: String,
    // pub metadata: serde_json::Value,
    pub status: String,
    pub last_interaction_at: String,
    pub updated_at: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbMemory {
    pub id: String,
    pub client: Client,
    pub interaction_id: String,
    pub conversation_id: String,
    pub flow_id: String,
    pub step_id: String,
    pub memory_order: i32,
    pub interaction_order: i32,
    pub key: String,
    pub value: serde_json::Value,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbMessage {
    pub id: String,
    pub client: Client,
    pub conversation_id: String,
    pub flow_id: String,
    pub step_id: String,
    pub message_order: i32,
    pub interaction_order: i32,
    pub direction: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbState {
    pub id: String,
    pub client: Client,
    #[serde(rename = "type")]
    pub _type: String,
    pub value: serde_json::Value,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbBot {
    pub id: String,
    pub bot_id: String,
    pub bot: String,
    pub engine_version: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BotVersion {
    pub bot: CsmlBot,
    pub version_id: String,
    pub engine_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BotVersionCreated {
    pub version_id: String,
    pub engine_version: String,
}

impl BotVersion {
    pub fn flatten(&self) -> serde_json::Value {
        let mut value = self.bot.to_json();

        value["version_id"] = serde_json::json!(self.version_id);
        value["engine_version"] = serde_json::json!(self.engine_version);

        value
    }
}
