use super::schema::*;
use crate::data;
use crate::data::EngineError;
use crate::db_connectors::diesel::Direction;
use crate::encrypt::decrypt_data;
use chrono::NaiveDateTime;
use csml_interpreter::data::Client;
use diesel::{Associations, Identifiable, Insertable, Queryable};
use std::convert::TryFrom;
use uuid::Uuid;

#[derive(Identifiable, Queryable, PartialEq, Debug)]
#[diesel(table_name = cmsl_bot_versions)]
pub struct Bot {
    pub id: Uuid,

    pub bot_id: String,
    pub bot: String,
    pub engine_version: String,

    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Insertable, Associations, PartialEq, Debug)]
#[diesel(table_name = cmsl_bot_versions, belongs_to(Bot))]
pub struct NewBot<'a> {
    pub id: Uuid,
    pub bot_id: &'a str,
    pub bot: &'a str,
    pub engine_version: &'a str,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[diesel(table_name = csml_conversations, belongs_to(Bot))]
pub struct Conversation {
    pub id: Uuid,

    pub bot_id: String,
    pub channel_id: String,
    pub user_id: String,

    pub flow_id: String,
    pub step_id: String,
    pub status: String,

    pub last_interaction_at: NaiveDateTime,

    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
}

impl From<Conversation> for data::models::Conversation {
    fn from(value: Conversation) -> Self {
        Self {
            id: value.id,
            client: Client {
                bot_id: value.bot_id,
                channel_id: value.channel_id,
                user_id: value.user_id,
            },
            flow_id: value.flow_id,
            step_id: value.step_id,
            status: value.status,
            last_interaction_at: value.last_interaction_at.and_utc(),
            updated_at: value.updated_at.and_utc(),
            created_at: value.created_at.and_utc(),
            expires_at: value.expires_at.as_ref().map(NaiveDateTime::and_utc),
        }
    }
}

#[derive(Insertable, Queryable, Associations, PartialEq, Debug)]
#[diesel(table_name = csml_conversations, belongs_to(Bot))]
pub struct NewConversation<'a> {
    pub id: Uuid,
    pub bot_id: &'a str,
    pub channel_id: &'a str,
    pub user_id: &'a str,

    pub flow_id: &'a str,
    pub step_id: &'a str,
    pub status: &'a str,

    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[diesel(table_name = csml_memories, belongs_to(Bot))]
pub struct Memory {
    pub id: Uuid,
    pub bot_id: String,
    pub channel_id: String,
    pub user_id: String,

    pub key: String,
    pub value: String,

    pub expires_at: Option<NaiveDateTime>,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Associations, PartialEq, Debug)]
#[diesel(table_name = csml_memories, belongs_to(Bot))]
pub struct NewMemory<'a> {
    pub id: Uuid,
    pub bot_id: &'a str,
    pub channel_id: &'a str,
    pub user_id: &'a str,

    pub key: &'a str,
    pub value: String, //serde_json::Value,

    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[diesel(table_name = csml_messages, belongs_to(Conversation))]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,

    pub flow_id: String,
    pub step_id: String,
    pub direction: Direction,
    pub payload: String,
    pub content_type: String,

    pub message_order: i32,
    pub interaction_order: i32,

    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,

    pub expires_at: Option<NaiveDateTime>,
}

impl TryFrom<Message> for data::models::Message {
    type Error = EngineError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        Ok(Self {
            id: message.id,
            conversation_id: message.conversation_id,
            flow_id: message.flow_id,
            step_id: message.step_id,
            direction: message.direction.into(),
            payload: decrypt_data(message.payload)?,
            content_type: message.content_type,
            message_order: message.message_order as u32,
            interaction_order: message.interaction_order as u32,
            updated_at: message.updated_at.and_utc(),
            created_at: message.created_at.and_utc(),
            expires_at: message.expires_at.as_ref().map(NaiveDateTime::and_utc),
        })
    }
}

#[derive(Insertable, Queryable, Associations, PartialEq, Debug)]
#[diesel(table_name = csml_messages, belongs_to(Conversation))]
pub struct NewMessages<'a> {
    pub id: Uuid,
    pub conversation_id: Uuid,

    pub flow_id: &'a str,
    pub step_id: &'a str,
    pub direction: Direction,
    pub payload: String,
    pub content_type: &'a str,

    pub message_order: i32,
    pub interaction_order: i32,

    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Identifiable, Insertable, Queryable, Associations, PartialEq, Debug)]
#[diesel(table_name = csml_states, belongs_to(Bot))]
pub struct State {
    pub id: Uuid,

    pub bot_id: String,
    pub channel_id: String,
    pub user_id: String,

    pub type_: String,
    pub key: String,
    pub value: String,

    pub expires_at: Option<NaiveDateTime>,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Associations, PartialEq, Debug)]
#[diesel(table_name = csml_states, belongs_to(Bot))]
pub struct NewState<'a> {
    pub id: Uuid,
    pub bot_id: &'a str,
    pub channel_id: &'a str,
    pub user_id: &'a str,

    pub type_: &'a str,
    pub key: &'a str,
    pub value: String,

    pub expires_at: Option<NaiveDateTime>,
}

// use serde::{ Deserializer};
// use serde_derive::{Serialize,Deserialize};

// const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

// fn datefmt<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
//     where
//         D: Deserializer<'de>,
// {
//     let s = String::deserialize(deserializer)?;
//     Utc.datetime_from_str(&s, FORMAT)
//         .map_err(serde::de::Error::custom)
// }

// fn option_datefmt<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
//     where
//         D: Deserializer<'de>,
// {
//     #[derive(Deserialize)]
//     struct Wrapper(#[serde(deserialize_with = "datefmt")] NaiveDateTime);

//     let v = Option::deserialize(deserializer)?;
//     Ok(v.map(|Wrapper(a)| a))
// }
