use crate::data::EngineError;
use chrono::{DateTime, Utc};
use csml_interpreter::data::{Client, CsmlBot, MultiBot};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlowTrigger {
    pub flow_id: String,
    pub step_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunRequest {
    pub bot: Option<CsmlBot>,
    pub bot_id: Option<String>,
    pub version_id: Option<String>,
    #[serde(alias = "fn_endpoint")]
    pub apps_endpoint: Option<String>,
    pub multibot: Option<Vec<MultiBot>>,
    pub event: CsmlRequest,
}

impl RunRequest {
    pub fn get_bot_opt(&self) -> Result<BotOpt, EngineError> {
        match self.clone() {
            // Bot
            RunRequest {
                bot: Some(mut csml_bot),
                multibot,
                ..
            } => {
                csml_bot.multibot = multibot;

                Ok(BotOpt::CsmlBot(csml_bot))
            }

            // version id
            RunRequest {
                version_id: Some(version_id),
                bot_id: Some(bot_id),
                apps_endpoint,
                multibot,
                ..
            } => Ok(BotOpt::Id {
                version_id,
                bot_id,
                apps_endpoint,
                multibot,
            }),

            // get bot by id will search for the last version id
            RunRequest {
                bot_id: Some(bot_id),
                apps_endpoint,
                multibot,
                ..
            } => Ok(BotOpt::BotId {
                bot_id,
                apps_endpoint,
                multibot,
            }),

            _ => Err(EngineError::Format("Invalid bot_opt format".to_owned())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BotOpt {
    #[serde(rename = "bot")]
    CsmlBot(CsmlBot),
    #[serde(rename = "version_id")]
    Id {
        version_id: String,
        bot_id: String,
        #[serde(alias = "fn_endpoint")]
        apps_endpoint: Option<String>,
        multibot: Option<Vec<MultiBot>>,
    },
    #[serde(rename = "bot_id")]
    BotId {
        bot_id: String,
        #[serde(alias = "fn_endpoint")]
        apps_endpoint: Option<String>,
        multibot: Option<Vec<MultiBot>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmlRequest {
    pub request_id: String,
    pub client: Client,
    pub callback_url: Option<String>,
    pub payload: serde_json::Value,
    pub metadata: serde_json::Value,
    pub step_limit: Option<usize>,
    pub ttl_duration: Option<serde_json::Value>,
    pub low_data_mode: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Conversation {
    pub id: Uuid,

    pub client: Client,

    pub flow_id: String,
    pub step_id: String,
    pub status: String,

    pub last_interaction_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Direction {
    Send,
    Receive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: Uuid,

    pub conversation_id: Uuid,
    pub flow_id: String,
    pub step_id: String,
    pub message_order: u32,
    pub interaction_order: u32,

    pub direction: Direction,
    pub content_type: String,
    pub payload: serde_json::Value,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaginationData {
    pub page: u32,
    pub total_pages: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paginated<T>
where
    T: serde::Serialize,
{
    pub data: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationData>,
}

// macro_rules! paginated {
//     ($name:ident, $field:ident, $data:ty) => {
//         #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
//         pub struct $name {
//             pub $field: $data,
//             #[serde(skip_serializing_if = "Option::is_none")]
//             pub pagination_key: Option<String>,
//         }
//     }
// }
//
// paginated!(PaginatedMessages, messages, Vec<Message>);
