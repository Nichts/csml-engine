pub mod db_connectors;
pub mod init;
pub mod utils;
// mod encrypt;
// mod error_messages;
pub mod interpreter_actions;
pub mod send;
// mod models;

pub use csml_interpreter::{
    data::{
        ast::{Expr, Flow, InstructionScope},
        csml_logs::*,
        error_info::ErrorInfo,
        position::Position,
        warnings::Warnings,
        Client, CsmlResult, Event,
    },
    load_components, search_for_modules,
};

use crate::data::*;
use crate::interpreter_actions::models::SwitchBot;
use db_connectors::{
    bot, clean_db, conversations, init_db, memories, messages, state,
    state::{delete_state_key, set_state_items},
    user,
};
use init::*;
use interpreter_actions::interpret_step;
use utils::*;

use crate::data;
use crate::data::filter::ClientMessageFilter;
use crate::data::models::{BotOpt, Conversation, CsmlRequest, Direction, Message, Paginated};
use crate::models::{BotVersion, BotVersionCreated};
use chrono::prelude::*;
use csml_interpreter::data::{csml_bot::CsmlBot, Hold, IndexInfo};
use futures::future::{BoxFuture, FutureExt};
use std::{collections::HashMap, env};
use uuid::Uuid;

pub async fn start_conversation_db(
    request: CsmlRequest,
    mut bot_opt: BotOpt,
    mut db: AsyncDatabase<'_>,
) -> Result<serde_json::Map<String, serde_json::Value>, EngineError> {
    init_logger();

    let mut formatted_event = format_event(&request)?;

    let mut bot = bot_opt.search_bot_async(&mut db).await?;
    init_bot(&mut bot)?;

    let mut data = init_conversation_info(
        get_default_flow(&bot)?.name.to_owned(),
        &formatted_event,
        &request,
        &bot,
        db,
    )
    .await?;

    check_for_hold(&mut data, &bot, &mut formatted_event).await?;

    /////////// block user event if delay variable si on and delay_time is bigger than current time
    if let Some(delay) = bot.no_interruption_delay {
        if let Some(delay) =
            state::get_state_key(&data.client, "delay", "content", &mut data.db).await?
        {
            match (delay["delay_value"].as_i64(), delay["timestamp"].as_i64()) {
                (Some(delay), Some(timestamp)) if timestamp + delay >= Utc::now().timestamp() => {
                    return Ok(serde_json::Map::new());
                }
                _ => {}
            }
        }

        let delay: serde_json::Value = serde_json::json!({
            "delay_value": delay,
            "timestamp": Utc::now().timestamp()
        });

        set_state_items(
            &data.client,
            "delay",
            vec![("content", &delay)],
            data.ttl,
            &mut data.db,
        )
        .await?;
    }
    //////////////////////////////////////

    // save event in db as message RECEIVE
    match (data.low_data, formatted_event.secure) {
        (false, true) => {
            let msgs = vec![serde_json::json!({"content_type": "secure"})];

            messages::add_messages_bulk(&mut data, msgs, 0, Direction::Receive).await?;
        }
        (false, false) => {
            let msgs = vec![request.payload];

            messages::add_messages_bulk(&mut data, msgs, 0, Direction::Receive).await?;
        }
        (true, _) => {}
    }

    let result = interpret_step(&mut data, formatted_event.to_owned(), &bot).await;

    check_switch_bot(
        result,
        &mut data,
        &mut bot,
        &mut bot_opt,
        &mut formatted_event,
    )
    .await
}

/**
 * Initiate a CSML chat request.
 * Takes 2 arguments: the request being made and the CSML bot.
 * This method assumes that the bot is already validated in advance. A best practice is
 * to pre-validate the bot and store it in a valid state.
 *
 * The request must be made by a given client. Its unicity (used as a key for identifying
 * who made each new request and if they relate to an already-open conversation) is based
 * on a combination of 3 parameters that are assumed to be unique in their own context:
 * - bot_id: differentiate bots handled by the same CSML engine instance
 * - channel_id: a given bot may be used on different channels (messenger, slack...)
 * - user_id: differentiate users on the same communication channel
 */
pub async fn start_conversation(
    request: CsmlRequest,
    bot_opt: BotOpt,
) -> Result<serde_json::Map<String, serde_json::Value>, EngineError> {
    let db = init_db().await?;
    start_conversation_db(request, bot_opt, db).await
}

fn check_switch_bot<'a>(
    result: Result<
        (
            serde_json::Map<String, serde_json::Value>,
            Option<SwitchBot>,
        ),
        EngineError,
    >,
    data: &'a mut AsyncConversationInfo<'a>,
    bot: &'a mut CsmlBot,
    bot_opt: &'a mut BotOpt,
    event: &'a mut Event,
) -> BoxFuture<'a, Result<serde_json::Map<String, serde_json::Value>, EngineError>> {
    async move {
        match result {
            Ok((mut messages, Some(next_bot))) => {
                if let Err(err) = switch_bot(data, bot, next_bot, bot_opt, event).await {
                    // End no interruption delay
                    if bot.no_interruption_delay.is_some() {
                        delete_state_key(&data.client, "delay", "content", &mut data.db).await?;
                    }
                    return Err(err);
                };

                let result = interpret_step(data, event.clone(), bot).await;

                let mut new_messages = check_switch_bot(result, data, bot, bot_opt, event).await?;

                messages.append(&mut new_messages);

                Ok(messages)
            }
            Ok((messages, None)) => {
                // End no interruption delay
                if bot.no_interruption_delay.is_some() {
                    delete_state_key(&data.client, "delay", "content", &mut data.db).await?;
                }

                Ok(messages)
            }
            Err(err) => {
                // End no interruption delay
                if bot.no_interruption_delay.is_some() {
                    delete_state_key(&data.client, "delay", "content", &mut data.db).await?;
                }

                Err(err)
            }
        }
    }
    .boxed()
}

/**
 * Return the latest conversation that is still open for a given user
 * (there should not be more than one), or None if there isn't any.
 */
pub async fn get_open_conversation(client: &Client) -> Result<Option<Conversation>, EngineError> {
    let mut db = init_db().await?;
    init_logger();

    conversations::get_latest_open(client, &mut db).await
}

pub async fn get_client_memories(client: &Client) -> Result<serde_json::Value, EngineError> {
    let mut db = init_db().await?;
    init_logger();

    memories::get_memories(client, &mut db).await
}

pub async fn get_client_memory(
    client: &Client,
    key: &str,
) -> Result<serde_json::Value, EngineError> {
    let mut db = init_db().await?;
    init_logger();

    memories::get_memory(client, key, &mut db).await
}

#[deprecated]
pub async fn get_client_messages(
    filter: ClientMessageFilter<'_>,
) -> Result<Paginated<Message>, EngineError> {
    let mut db = init_db().await?;

    get_client_messages_filtered(&mut db, filter).await
}

pub async fn get_client_messages_filtered<'conn, 'a: 'conn>(
    db: &'a mut AsyncDatabase<'conn>,
    filter: ClientMessageFilter<'a>,
) -> Result<Paginated<Message>, EngineError> {
    init_logger();

    messages::get_client_messages(db, filter).await
}

pub async fn get_conversation<'conn, 'a: 'conn>(
    db: &'a mut AsyncDatabase<'conn>,
    id: Uuid,
) -> Result<data::models::Conversation, EngineError> {
    init_logger();

    conversations::get_conversation(db, id).await
}

pub async fn get_client_conversations(
    client: &Client,
    limit: Option<u32>,
    pagination_key: Option<u32>,
) -> Result<Paginated<Conversation>, EngineError> {
    let mut db = init_db().await?;
    init_logger();

    conversations::get_client_conversations(client, &mut db, limit, pagination_key).await
}

/**
 * Get current State ether Hold or NULL
 */
pub async fn get_current_state(client: &Client) -> Result<Option<serde_json::Value>, EngineError> {
    let mut db = init_db().await?;
    init_logger();

    state::get_current_state(client, &mut db).await
}

/**
 * Create memory
 */
pub async fn create_client_memory(
    client: &Client,
    key: String,
    value: serde_json::Value,
) -> Result<(), EngineError> {
    let mut db = init_db().await?;
    init_logger();
    validate_memory_key_format(&key)?;

    let ttl = get_ttl_duration_value(None);

    memories::create_client_memory(client, key, value, ttl, &mut db).await
}

/**
 * Create bot version
 */
pub async fn create_bot_version(mut csml_bot: CsmlBot) -> Result<BotVersionCreated, EngineError> {
    let mut db = init_db().await?;
    init_logger();

    let bot_id = csml_bot.id.clone();

    // search for modules to download
    if let Err(err) = search_for_modules(&mut csml_bot) {
        return Err(EngineError::Interpreter(format!("{:?}", err)));
    }

    match validate_bot(csml_bot.clone()) {
        CsmlResult {
            errors: Some(errors),
            ..
        } => Err(EngineError::Interpreter(format!("{:?}", errors))),
        CsmlResult { .. } => {
            let version_id = bot::create_bot_version(bot_id, csml_bot, &mut db).await?;
            let engine_version = env!("CARGO_PKG_VERSION").to_owned();

            Ok(BotVersionCreated {
                version_id,
                engine_version,
            })
        }
    }
}

/**
 * get by bot_id
 */
pub async fn get_last_bot_version(bot_id: &str) -> Result<Option<BotVersion>, EngineError> {
    let mut db = init_db().await?;
    init_logger();

    bot::get_last_bot_version(bot_id, &mut db).await
}

/**
 * get bot by version_id
 */
pub async fn get_bot_by_version_id(
    id: &str,
    bot_id: &str,
) -> Result<Option<BotVersion>, EngineError> {
    let mut db = init_db().await?;
    init_logger();

    bot::get_by_version_id(id, bot_id, &mut db).await
}

/**
 * List the last 20 versions of the bot if no limit is set
 *
 * BOT = {
 *  "version_id": String,
 *  "id": String,
 *  "name": String,
 *  "custom_components": Option<String>,
 *  "default_flow": String
 *  "engine_version": String
 *  "created_at": String
 * }
 */
pub async fn get_bot_versions(
    bot_id: &str,
    limit: Option<u32>,
    last_key: Option<u32>,
) -> Result<serde_json::Value, EngineError> {
    let mut db = init_db().await?;
    init_logger();

    bot::get_bot_versions(bot_id, limit, last_key, &mut db).await
}

/**
 * delete bot by version_id
 */
pub async fn delete_bot_version_id(id: &str, bot_id: &str) -> Result<(), EngineError> {
    let mut db = init_db().await?;
    init_logger();

    bot::delete_bot_version(bot_id, id, &mut db).await
}

/**
 * Delete all bot versions of bot_id
 */
pub async fn delete_all_bot_versions(bot_id: &str) -> Result<(), EngineError> {
    let mut db = init_db().await?;
    init_logger();

    bot::delete_bot_versions(bot_id, &mut db).await
}

/**
 * Delete all data related to bot: versions, conversations, messages, memories, nodes, integrations
 */
pub async fn delete_all_bot_data(bot_id: &str) -> Result<(), EngineError> {
    let mut db = init_db().await?;
    init_logger();

    bot::delete_all_bot_data(bot_id, &mut db).await
}

/**
 * Delete all the memories of a given client
 */
pub async fn delete_client_memories(client: &Client) -> Result<(), EngineError> {
    let mut db = init_db().await?;
    init_logger();

    memories::delete_client_memories(client, &mut db).await
}

/**
 * Delete a single memory for a given Client
 */
pub async fn delete_client_memory(client: &Client, memory_name: &str) -> Result<(), EngineError> {
    let mut db = init_db().await?;
    init_logger();

    memories::delete_client_memory(client, memory_name, &mut db).await
}

/**
 * Delete all data related to a given Client
 */
pub async fn delete_client(client: &Client) -> Result<(), EngineError> {
    let mut db = init_db().await?;
    init_logger();

    user::delete_client(client, &mut db).await
}

/**
 * List all the steps in every flow of a given CSML bot
 */
pub fn get_steps_from_flow(bot: CsmlBot) -> HashMap<String, Vec<String>> {
    csml_interpreter::get_steps_from_flow(bot)
}

/**
 * Simple static CSML bot linter.
 * Does not check for possible runtime errors, only for build-time errors
 * (missing steps or flows, syntax errors, etc.)
 */
pub fn validate_bot(mut bot: CsmlBot) -> CsmlResult {
    // load native components into the bot
    bot.native_components = match load_components() {
        Ok(components) => Some(components),
        Err(err) => {
            return CsmlResult {
                errors: Some(vec![err]),
                warnings: None,
                flows: None,
                extern_flows: None,
            };
        }
    };

    // search for modules to download
    if let Err(err) = search_for_modules(&mut bot) {
        return CsmlResult {
            errors: Some(vec![ErrorInfo::new(Position::default(), err)]),
            warnings: None,
            flows: None,
            extern_flows: None,
        };
    }

    csml_interpreter::validate_bot(&bot)
}

/**
 * fold CSML bot in one single flow.
 * Rename all existing steps, goto and functions in order to match their origin flow.
 * Examples:
 *  step_name: -> flow_name_step_name:
 *  goto step_name -> goto flow_name_step_name
 */
pub fn fold_bot(mut bot: CsmlBot) -> Result<String, EngineError> {
    // load native components into the bot
    bot.native_components = match load_components() {
        Ok(components) => Some(components),
        Err(err) => return Err(EngineError::Parring(err.format_error())),
    };

    Ok(csml_interpreter::fold_bot(&bot))
}

/**
 * Close any open conversation a given client may currently have.
 * We also need to both clean the hold/local memory state to make sure
 * that outdated variables or hold positions are not loaded into the next open conversation.
 */
pub async fn user_close_all_conversations(client: Client) -> Result<(), EngineError> {
    let mut db = init_db().await?;
    init_logger();

    state::delete_state_key(&client, "hold", "position", &mut db).await?;
    conversations::close_all_conversations(&client, &mut db).await
}

/**
 * Verify if the user is currently on hold in a given conversation.
 *
 * If a hold is found, make sure that the flow has not been updated since last conversation.
 * If that's the case, we can not be sure that the hold is in the same position,
 * so we need to clear the hold's position and restart the conversation.
 *
 * If the hold is valid, we also need to load the local step memory
 * (context.hold.step_vars) into the conversation context.
 */
async fn check_for_hold(
    data: &mut AsyncConversationInfo<'_>,
    bot: &CsmlBot,
    event: &mut Event,
) -> Result<(), EngineError> {
    match state::get_state_key(&data.client, "hold", "position", &mut data.db).await {
        // user is currently on hold
        Ok(Some(hold)) => {
            match hold.get("hash") {
                Some(hash_value) => {
                    let flow_hash = get_current_step_hash(&data.context, bot)?;
                    // cleanup the current hold and restart flow
                    if flow_hash != *hash_value {
                        return clean_hold_and_restart(data).await;
                    }
                    flow_hash
                }
                _ => return Ok(()),
            };

            let index = match serde_json::from_value::<IndexInfo>(hold["index"].clone()) {
                Ok(index) => index,
                Err(_) => {
                    state::delete_state_key(&data.client, "hold", "position", &mut data.db).await?;
                    return Ok(());
                }
            };

            let secure_hold = hold["secure"].as_bool().unwrap_or(false);

            if secure_hold {
                event.secure = true;
            }

            // all good, let's load the position and local variables
            data.context.hold = Some(Hold {
                index,
                step_vars: hold["step_vars"].clone(),
                step_name: data.context.step.get_step(),
                flow_name: data.context.flow.to_owned(),
                previous: serde_json::from_value(hold["previous"].clone()).unwrap_or(None),
                secure: secure_hold,
            });

            delete_state_key(&data.client, "hold", "position", &mut data.db).await?;
        }
        // user is not on hold
        Ok(None) => (),
        Err(_) => (),
    };
    Ok(())
}

/**
 * get server status
 */
pub async fn get_status() -> Result<serde_json::Value, EngineError> {
    let mut status = serde_json::Map::new();

    let mut ready = true;

    match std::env::var("ENGINE_DB_TYPE") {
        Ok(db_name) => match init_db().await {
            Ok(_) => status.insert("database_type".to_owned(), serde_json::json!(db_name)),
            Err(_) => {
                ready = false;
                status.insert(
                    "database_type".to_owned(),
                    serde_json::json!(format!("Setup error: {}", db_name)),
                )
            }
        },
        Err(_) => {
            ready = false;
            status.insert(
                "database_type".to_owned(),
                serde_json::json!("error: no database type selected"),
            )
        }
    };

    match ready {
        true => status.insert("server_ready".to_owned(), serde_json::json!(true)),
        false => status.insert("server_ready".to_owned(), serde_json::json!(false)),
    };

    match std::env::var("ENGINE_SERVER_PORT") {
        Ok(port) => status.insert("server_port".to_owned(), serde_json::json!(port)),
        Err(_) => status.insert("server_port".to_owned(), serde_json::json!(5000)), // DEFAULT
    };

    match std::env::var("ENGINE_SERVER_API_KEYS") {
        Ok(_) => status.insert("server_auth_enabled".to_owned(), serde_json::json!(true)),
        Err(_) => status.insert("server_auth_enabled".to_owned(), serde_json::json!(false)),
    };

    match std::env::var("ENCRYPTION_SECRET") {
        Ok(_) => status.insert("encryption_enabled".to_owned(), serde_json::json!(true)),
        Err(_) => status.insert("encryption_enabled".to_owned(), serde_json::json!(false)),
    };

    match std::env::var("DEBUG") {
        Ok(_) => status.insert("debug_mode_enabled".to_owned(), serde_json::json!(true)),
        Err(_) => status.insert("debug_mode_enabled".to_owned(), serde_json::json!(false)),
    };

    match std::env::var("CSML_LOG_LEVEL") {
        Ok(val) => status.insert("csml_log_level".to_owned(), serde_json::json!(val)),
        Err(_) => status.insert(
            "csml_log_level".to_owned(),
            serde_json::json!("info".to_owned()),
        ),
    };

    status.insert(
        "engine_version".to_owned(),
        serde_json::json!(env!("CARGO_PKG_VERSION")),
    );

    Ok(serde_json::json!(status))
}

/**
 * delete expired data
 */
pub async fn delete_expired_data() -> Result<(), EngineError> {
    let mut db = init_db().await?;

    clean_db::delete_expired_data(&mut db).await
}
