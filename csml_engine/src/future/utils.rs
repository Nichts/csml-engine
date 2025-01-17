use crate::{
    data::{AsyncConversationInfo, AsyncDatabase, EngineError},
    future::db_connectors::state::delete_state_key,
    future::send::send_to_callback_url,
    CsmlBot, CsmlFlow,
};

use base64::Engine;
use chrono::{prelude::Utc, SecondsFormat};
use csml_interpreter::{
    data::{
        ast::{Flow, InsertStep, InstructionScope},
        context::ContextStepInfo,
        csml_logs::*,
        Client, Context, Event, Interval, Memory, Message,
    },
    error_format::{ERROR_KEY_ALPHANUMERIC, ERROR_NUMBER_AS_KEY, ERROR_SIZE_IDENT},
    get_step,
    interpreter::json_to_literal,
};
use rand::seq::SliceRandom;
use serde_json::{json, map::Map, Value};
use std::collections::HashMap;
use std::env;

use crate::data::models::{CsmlRequest, FlowTrigger};
use md5::{Digest, Md5};
use regex::Regex;

/**
 * Update current context memories in place.
 * This method is used to avoid saving memories in DB every time a `remember` is used
 * Instead, the memory is saved in bulk at the end of each step or interaction, but we still
 * must allow the user to use the `remembered` data immediately.
 */
pub fn update_current_context(
    data: &mut AsyncConversationInfo<'_>,
    memories: &HashMap<String, Memory>,
) {
    for (_key, mem) in memories.iter() {
        let lit = json_to_literal(&mem.value, Interval::default(), &data.context.flow).unwrap();

        data.context.current.insert(mem.key.to_owned(), lit);
    }
}

/**
 * Check if memory key is valid
 */
pub fn validate_memory_key_format(key: &str) -> Result<(), EngineError> {
    if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(EngineError::Format(ERROR_KEY_ALPHANUMERIC.to_owned()));
    }

    if key.len() > std::u8::MAX as usize {
        return Err(EngineError::Format(ERROR_SIZE_IDENT.to_owned()));
    }

    if key.parse::<f64>().is_ok() {
        return Err(EngineError::Format(ERROR_NUMBER_AS_KEY.to_owned()));
    }

    Ok(())
}

/**
 * Prepare a formatted "content" for the event object, based on the user's input.
 * This will trim extra data and only keep the main value.
 */
pub fn get_event_content(content_type: &str, metadata: &Value) -> Result<String, EngineError> {
    match content_type {
        file if ["file", "audio", "video", "image", "url"].contains(&file) => {
            if let Some(val) = metadata["url"].as_str() {
                Ok(val.to_string())
            } else {
                Err(EngineError::Interpreter(
                    "no url content in event".to_owned(),
                ))
            }
        }
        payload if payload == "payload" => {
            if let Some(val) = metadata["payload"].as_str() {
                Ok(val.to_string())
            } else {
                Err(EngineError::Interpreter(
                    "no payload content in event".to_owned(),
                ))
            }
        }
        text if text == "text" => {
            if let Some(val) = metadata["text"].as_str() {
                Ok(val.to_string())
            } else {
                Err(EngineError::Interpreter(
                    "no text content in event".to_owned(),
                ))
            }
        }
        regex if regex == "regex" => {
            if let Some(val) = metadata["payload"].as_str() {
                Ok(val.to_string())
            } else {
                Err(EngineError::Interpreter(
                    "invalid payload for event type regex".to_owned(),
                ))
            }
        }
        flow_trigger if flow_trigger == "flow_trigger" => {
            match serde_json::from_value::<FlowTrigger>(metadata.clone()) {
                Ok(_flow_trigger) => {
                    Ok(metadata.to_string())
                }
                Err(_) => {
                    Err(EngineError::Interpreter(
                        "invalid content for event type flow_trigger: expect flow_id and optional step_id".to_owned(),
                    ))
                }
            }
        }
        content_type => Err(EngineError::Interpreter(format!(
            "{} is not a valid content_type",
            content_type
        ))),
    }
}

/**
 * Format the incoming (JSON-formatted) event into an Event struct.
 */
pub fn format_event(request: &CsmlRequest) -> Result<Event, EngineError> {
    let step_limit = request.step_limit;
    let json_event = json!(request);

    let content_type = match json_event["payload"]["content_type"].as_str() {
        Some(content_type) => content_type.to_string(),
        None => {
            return Err(EngineError::Interpreter(
                "no content_type in event payload".to_owned(),
            ))
        }
    };
    let content = json_event["payload"]["content"].to_owned();

    let content_value = get_event_content(&content_type, &content)?;

    Ok(Event {
        content_type,
        content_value,
        content,
        ttl_duration: json_event["ttl_duration"].as_i64(),
        low_data_mode: json_event["low_data_mode"].as_bool(),
        step_limit,
        secure: json_event["payload"]["secure"].as_bool().unwrap_or(false),
    })
}

/**
 * Send a message to the configured callback_url.
 * If not callback_url is configured, skip this action.
 */
pub async fn send_msg_to_callback_url(
    data: &mut AsyncConversationInfo<'_>,
    msg: Vec<Message>,
    interaction_order: i32,
    end: bool,
) {
    let messages = messages_formatter(data, msg, interaction_order, end);

    csml_logger(
        CsmlLog::new(
            Some(&data.client),
            Some(data.context.flow.to_string()),
            None,
            format!("conversation_end: {:?}", messages["conversation_end"]),
        ),
        LogLvl::Debug,
    );

    send_to_callback_url(data, serde_json::json!(messages)).await
}

/**
 * Update ConversationInfo data with current information about the request.
 */
fn add_info_to_message(
    data: &AsyncConversationInfo<'_>,
    mut msg: Message,
    interaction_order: i32,
) -> Value {
    let payload = msg.message_to_json();

    let mut map_msg: Map<String, Value> = Map::new();
    map_msg.insert("payload".to_owned(), payload);
    map_msg.insert("interaction_order".to_owned(), json!(interaction_order));
    map_msg.insert("conversation_id".to_owned(), json!(data.conversation_id));
    map_msg.insert("direction".to_owned(), json!("SEND"));

    Value::Object(map_msg)
}

/**
 * Prepare correctly formatted messages as requested in both:
 * - send action: when callback_url is set, messages are sent as they come to a defined endpoint
 * - return action: at the end of the interaction, all messages are returned as they were processed
 */
pub fn messages_formatter(
    data: &mut AsyncConversationInfo<'_>,
    vec_msg: Vec<Message>,
    interaction_order: i32,
    end: bool,
) -> Map<String, Value> {
    let msgs = vec_msg
        .into_iter()
        .map(|msg| add_info_to_message(data, msg, interaction_order))
        .collect();
    let mut map: Map<String, Value> = Map::new();

    map.insert("messages".to_owned(), Value::Array(msgs));
    map.insert("conversation_end".to_owned(), Value::Bool(end));
    map.insert("request_id".to_owned(), json!(data.request_id));

    map.insert(
        "received_at".to_owned(),
        json!(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
    );

    let mut map_client: Map<String, Value> = Map::new();

    map_client.insert("bot_id".to_owned(), json!(data.client.bot_id));
    map_client.insert("user_id".to_owned(), json!(data.client.user_id));
    map_client.insert("channel_id".to_owned(), json!(data.client.channel_id));

    map.insert("client".to_owned(), Value::Object(map_client));

    map
}

/**
 * Retrieve a flow in a given bot by an identifier:
 * - matching method is case insensitive
 * - as name is similar to a flow's alias, both flow.name and flow.id can be matched.
 */
pub fn get_flow_by_id<'a>(f_id: &str, flows: &'a [CsmlFlow]) -> Result<&'a CsmlFlow, EngineError> {
    let id = f_id.to_ascii_lowercase();
    // TODO: move to_lowercase at creation of vars
    match flows
        .iter()
        .find(|&val| val.id.to_ascii_lowercase() == id || val.name.to_ascii_lowercase() == id)
    {
        Some(f) => Ok(f),
        None => Err(EngineError::Interpreter(format!(
            "Flow '{}' does not exist",
            f_id
        ))),
    }
}

/**
 * Retrieve a bot's default flow.
 * The default flow must exist!
 */
pub fn get_default_flow<'a>(bot: &'a CsmlBot) -> Result<&'a CsmlFlow, EngineError> {
    match bot
        .flows
        .iter()
        .find(|&flow| flow.id == bot.default_flow || flow.name == bot.default_flow)
    {
        Some(flow) => Ok(flow),
        None => Err(EngineError::Interpreter(
            "The bot's default_flow does not exist".to_owned(),
        )),
    }
}

/**
 * Find a flow in a bot based on the user's input.
 * - flow_trigger events must will match a flow's id or name and reset the hold position
 * - other events will try to match a flow trigger
 */
pub async fn search_flow<'a>(
    event: &Event,
    bot: &'a CsmlBot,
    client: &Client,
    db: &mut AsyncDatabase<'_>,
) -> Result<(&'a CsmlFlow, String), EngineError> {
    match event {
        event if event.content_type == "flow_trigger" => {
            delete_state_key(client, "hold", "position", db).await?;

            let flow_trigger: FlowTrigger = serde_json::from_str(&event.content_value)?;

            match get_flow_by_id(&flow_trigger.flow_id, &bot.flows) {
                Ok(flow) => match flow_trigger.step_id {
                    Some(step_id) => Ok((flow, step_id)),
                    None => Ok((flow, "start".to_owned())),
                },
                Err(_) => Ok((
                    get_flow_by_id(&bot.default_flow, &bot.flows)?,
                    "start".to_owned(),
                )),
            }
        }
        event if event.content_type == "regex" => {
            let mut random_flows = vec![];

            for flow in bot.flows.iter() {
                let contains_command = flow.commands.iter().any(|cmd| {
                    if let Ok(action) = Regex::new(&event.content_value) {
                        action.is_match(cmd)
                    } else {
                        false
                    }
                });

                if contains_command {
                    random_flows.push(flow)
                }
            }

            // Thread rng is not send so we have to drop it right away
            let random_flow = {
                let rng = &mut rand::thread_rng();
                random_flows.choose(rng)
            };

            match random_flow {
                Some(flow) => {
                    delete_state_key(client, "hold", "position", db).await?;
                    Ok((flow, "start".to_owned()))
                }
                None => Err(EngineError::Interpreter(format!(
                    "no match found for regex: {}",
                    event.content_value
                ))),
            }
        }
        event => {
            let mut random_flows = vec![];

            for flow in bot.flows.iter() {
                let contains_command = flow
                    .commands
                    .iter()
                    .any(|cmd| &cmd.as_str().to_lowercase() == &event.content_value.to_lowercase());

                if contains_command {
                    random_flows.push(flow)
                }
            }

            // Thread rng is not send so we have to drop it right away
            let random_flow = {
                let rng = &mut rand::thread_rng();
                random_flows.choose(rng)
            };

            match random_flow {
                Some(flow) => {
                    delete_state_key(client, "hold", "position", db).await?;
                    Ok((flow, "start".to_owned()))
                }
                None => Err(EngineError::Interpreter(format!(
                    "Flow '{}' does not exist",
                    event.content_value
                ))),
            }
        }
    }
}

pub fn get_current_step_hash(context: &Context, bot: &CsmlBot) -> Result<String, EngineError> {
    let mut hash = Md5::new();

    let step = match &context.step {
        ContextStepInfo::Normal(step) => {
            let flow = &get_flow_by_id(&context.flow, &bot.flows)?.content;

            let ast = match &bot.bot_ast {
                Some(ast) => {
                    let base64decoded = base64::engine::general_purpose::STANDARD
                        .decode(ast)
                        .unwrap();
                    let csml_bot: HashMap<String, Flow> =
                        bincode::deserialize(&base64decoded[..]).unwrap();
                    match csml_bot.get(&context.flow) {
                        Some(flow) => flow.to_owned(),
                        None => csml_bot
                            .get(&get_default_flow(bot)?.name)
                            .unwrap()
                            .to_owned(),
                    }
                }
                None => return Err(EngineError::Manager("not valid ast".to_string())),
            };

            get_step(step, flow, &ast)
        }
        ContextStepInfo::UnknownFlow(step) => {
            let flow = &get_flow_by_id(&context.flow, &bot.flows)?.content;

            match &bot.bot_ast {
                Some(ast) => {
                    let base64decoded = base64::engine::general_purpose::STANDARD
                        .decode(ast)
                        .unwrap();
                    let csml_bot: HashMap<String, Flow> =
                        bincode::deserialize(&base64decoded[..]).unwrap();

                    let default_flow = csml_bot.get(&get_default_flow(bot)?.name).unwrap();

                    match csml_bot.get(&context.flow) {
                        Some(target_flow) => {
                            // check if there is a inserted step with the same name as the target step
                            let insertion_expr = target_flow.flow_instructions.get_key_value(
                                &InstructionScope::InsertStep(InsertStep {
                                    name: step.clone(),
                                    original_name: None,
                                    from_flow: "".to_owned(),
                                    interval: Interval::default(),
                                }),
                            );

                            // if there is a inserted step get the flow of the target step and
                            if let Some((InstructionScope::InsertStep(insert), _)) = insertion_expr
                            {
                                match csml_bot.get(&insert.from_flow) {
                                    Some(inserted_step_flow) => {
                                        let inserted_raw_flow =
                                            &get_flow_by_id(&insert.from_flow, &bot.flows)?.content;

                                        get_step(step, inserted_raw_flow, inserted_step_flow)
                                    }
                                    None => get_step(step, flow, default_flow),
                                }
                            } else {
                                get_step(step, flow, target_flow)
                            }
                        }
                        None => get_step(step, flow, default_flow),
                    }
                }
                None => return Err(EngineError::Manager("not valid ast".to_string())),
            }
        }
        ContextStepInfo::InsertedStep {
            step,
            flow: inserted_flow,
        } => {
            let flow = &get_flow_by_id(inserted_flow, &bot.flows)?.content;

            let ast = match &bot.bot_ast {
                Some(ast) => {
                    let base64decoded = base64::engine::general_purpose::STANDARD
                        .decode(ast)
                        .unwrap();
                    let csml_bot: HashMap<String, Flow> =
                        bincode::deserialize(&base64decoded[..]).unwrap();

                    match csml_bot.get(inserted_flow) {
                        Some(flow) => flow.to_owned(),
                        None => csml_bot
                            .get(&get_default_flow(bot)?.name)
                            .unwrap()
                            .to_owned(),
                    }
                }
                None => return Err(EngineError::Manager("not valid ast".to_string())),
            };

            get_step(step, flow, &ast)
        }
    };

    hash.update(step.as_bytes());

    Ok(format!("{:x}", hash.finalize()))
}

pub async fn clean_hold_and_restart(
    data: &mut AsyncConversationInfo<'_>,
) -> Result<(), EngineError> {
    delete_state_key(&data.client, "hold", "position", &mut data.db).await?;
    data.context.hold = None;
    Ok(())
}

pub fn get_ttl_duration_value(event: Option<&Event>) -> Option<chrono::Duration> {
    if let Some(event) = event {
        if let Some(ttl) = event.ttl_duration {
            return Some(chrono::Duration::days(ttl));
        }
    }

    if let Ok(ttl) = env::var("TTL_DURATION") {
        if let Ok(ttl) = ttl.parse::<i64>() {
            return Some(chrono::Duration::days(ttl));
        }
    }

    None
}

pub fn get_low_data_mode_value(event: &Event) -> bool {
    if let Some(low_data) = event.low_data_mode {
        return low_data;
    }

    if let Ok(low_data) = env::var("LOW_DATA_MODE") {
        if let Ok(low_data) = low_data.parse::<bool>() {
            return low_data;
        }
    }

    false
}
