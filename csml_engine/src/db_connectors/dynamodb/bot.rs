use crate::data::{DynamoBot, DynamoBotBincode, DynamoDbClient};
use crate::db_connectors::dynamodb::{aws_s3, Bot, BotKeys, Class, DynamoDbKey};
use crate::models::BotVersion;
use crate::EngineError;
use base64::Engine;
use csml_interpreter::data::{csml_bot::Module, csml_flow::CsmlFlow};
use rusoto_dynamodb::*;
use std::collections::HashMap;

pub fn create_bot_version(
    bot_id: String,
    bot: String,
    flows: String,
    flow_modules: String,
    db: &mut DynamoDbClient,
) -> Result<String, EngineError> {
    let data: Bot = Bot::new(bot_id, bot);

    let input = PutItemInput {
        item: serde_dynamodb::to_hashmap(&data)?,
        table_name: get_table_name()?,
        ..Default::default()
    };

    let client = db.client.to_owned();
    let future = client.put_item(input);
    db.runtime.block_on(future)?;

    let key = format!("bots/{}/versions/{}/flows.json", &data.id, &data.version_id);
    aws_s3::put_object(db, &key, flows)?;

    let key = format!(
        "bots/{}/versions/{}/modules.json",
        &data.id, &data.version_id
    );
    aws_s3::put_object(db, &key, flow_modules)?;

    Ok(data.version_id.to_owned())
}

pub fn get_flows(key: &str, db: &mut DynamoDbClient) -> Result<Vec<CsmlFlow>, EngineError> {
    let object = aws_s3::get_object(db, key)?;
    let flows: Vec<CsmlFlow> = match serde_json::from_str(&object) {
        Ok(flows) => flows,
        Err(_) => vec![],
    };

    Ok(flows)
}

pub fn get_modules(key: &str, db: &mut DynamoDbClient) -> Result<Vec<Module>, EngineError> {
    let object = match aws_s3::get_object(db, key) {
        Ok(obj) => obj,
        Err(_) => return Ok(vec![]), // for Backward compatibility create empty modules
    };

    let modules: Vec<Module> = match serde_json::from_str(&object) {
        Ok(modules) => modules,
        Err(_) => vec![],
    };

    Ok(modules)
}

fn query_bot_version(
    bot_id: &str,
    limit: i64,
    pagination_key: Option<HashMap<String, AttributeValue>>,
    db: &mut DynamoDbClient,
) -> Result<QueryOutput, EngineError> {
    let key_cond_expr =
        "#hashKey = :hashVal AND begins_with(#rangeTimeKey, :rangePrefix)".to_string();
    let expr_attr_names = [
        (String::from("#hashKey"), String::from("hash")),
        (String::from("#rangeTimeKey"), String::from("range_time")),
        (String::from("#rangeKey"), String::from("range")),
    ]
    .iter()
    .cloned()
    .collect();

    let expr_attr_values = [
        (
            String::from(":hashVal"),
            AttributeValue {
                s: Some(Bot::get_hash(bot_id)),
                ..Default::default()
            },
        ),
        (
            String::from(":rangePrefix"),
            AttributeValue {
                s: Some(String::from("bot#")),
                ..Default::default()
            },
        ),
    ]
    .iter()
    .cloned()
    .collect();

    let input = QueryInput {
        table_name: get_table_name()?,
        index_name: Some(String::from("TimeIndex")),
        key_condition_expression: Some(key_cond_expr),
        expression_attribute_names: Some(expr_attr_names),
        expression_attribute_values: Some(expr_attr_values),
        limit: Some(limit),
        select: Some(String::from("SPECIFIC_ATTRIBUTES")),
        projection_expression: Some("#hashKey, #rangeKey".to_owned()),
        scan_index_forward: Some(false),
        exclusive_start_key: pagination_key,
        ..Default::default()
    };

    let query = db.client.query(input);
    let data = match db.runtime.block_on(query) {
        Ok(data) => data,
        Err(e) => return Err(EngineError::Manager(format!("query_bot_version {:?}", e))),
    };

    Ok(data)
}

pub fn get_bot_versions(
    bot_id: &str,
    limit: Option<i64>,
    pagination_key: Option<HashMap<String, AttributeValue>>,
    db: &mut DynamoDbClient,
) -> Result<serde_json::Value, EngineError> {
    let limit = match limit {
        Some(limit) if limit >= 1 => limit,
        Some(_limit) => 20,
        None => 20,
    };

    let data = query_bot_version(bot_id, limit, pagination_key, db)?;
    /////////
    // The query returns an array of items (max 10, based on the limit param above).
    // If 0 item is returned it means that there is no open conversation, so simply return None
    // , "last_key": :
    let items = match data.items {
        None => return Ok(serde_json::json!({"bots": []})),
        Some(items) if items.len() == 0 => return Ok(serde_json::json!({"bots": []})),
        Some(items) => items.clone(),
    };

    let mut get_requests = vec![];

    for item in items {
        let bot_keys: BotKeys = serde_dynamodb::from_hashmap(item)?;

        let key = serde_dynamodb::to_hashmap(&DynamoDbKey {
            hash: bot_keys.hash,
            range: bot_keys.range,
        })?;

        get_requests.push(key);
    }

    let request_items = [(get_table_name()?, get_requests)]
        .iter()
        .cloned()
        .map(|(name, keys)| {
            let mut attval = KeysAndAttributes::default();

            attval.keys = keys;

            (name, attval)
        })
        .collect();

    let input = BatchGetItemInput {
        request_items,
        ..Default::default()
    };

    let bots = execute_bot_version_batch_get_query(db, input)?;

    match data.last_evaluated_key {
        Some(pagination_key) => {
            let pagination_key = base64::engine::general_purpose::STANDARD
                .encode(serde_json::json!(pagination_key).to_string());

            Ok(serde_json::json!({"bots": bots, "pagination_key": pagination_key}))
        }
        None => Ok(serde_json::json!({ "bots": bots })),
    }
}

pub fn get_bot_by_version_id(
    version_id: &str,
    bot_id: &str,
    db: &mut DynamoDbClient,
) -> Result<Option<BotVersion>, EngineError> {
    let item_key = DynamoDbKey {
        hash: Bot::get_hash(bot_id),
        range: Bot::get_range(version_id),
    };

    let input = GetItemInput {
        table_name: get_table_name()?,
        key: serde_dynamodb::to_hashmap(&item_key)?,
        ..Default::default()
    };

    let future = db.client.get_item(input);
    let res = db.runtime.block_on(future)?;

    match res.item {
        Some(val) => {
            let bot: Bot = serde_dynamodb::from_hashmap(val)?;

            let csml_bot: DynamoBot =
                match base64::engine::general_purpose::STANDARD.decode(&bot.bot) {
                    Ok(base64decoded) => {
                        match bincode::deserialize::<DynamoBotBincode>(&base64decoded[..]) {
                            Ok(bot) => bot.to_bot(),
                            Err(_) => serde_json::from_str(&bot.bot).unwrap(),
                        }
                    }
                    Err(_) => serde_json::from_str(&bot.bot).unwrap(),
                };

            let key = format!("bots/{}/versions/{}/flows.json", bot_id, version_id);
            let flows = get_flows(&key, db)?;

            let key = format!("bots/{}/versions/{}/modules.json", bot_id, version_id);
            let modules = get_modules(&key, db)?;

            Ok(Some(BotVersion {
                bot: csml_bot.to_bot(flows, modules),
                version_id: bot.version_id,
                engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            }))
        }
        _ => Ok(None),
    }
}

pub fn get_last_bot_version(
    bot_id: &str,
    db: &mut DynamoDbClient,
) -> Result<Option<BotVersion>, EngineError> {
    let expr_attr_names = [
        (String::from("#hashKey"), String::from("hash")),
        (String::from("#rangeKey"), String::from("range")),
        (String::from("#rangeTimeKey"), String::from("range_time")),
    ]
    .iter()
    .cloned()
    .collect();

    let expr_attr_values = [
        (
            String::from(":hashVal"),
            AttributeValue {
                s: Some(Bot::get_hash(bot_id)),
                ..Default::default()
            },
        ),
        (
            String::from(":rangePrefix"),
            AttributeValue {
                s: Some(String::from("bot#")),
                ..Default::default()
            },
        ),
    ]
    .iter()
    .cloned()
    .collect();

    let key_cond_expr =
        "#hashKey = :hashVal AND begins_with(#rangeTimeKey, :rangePrefix)".to_string();

    // retrieve last bot version from dynamodb
    let input = QueryInput {
        table_name: get_table_name()?,
        index_name: Some(String::from("TimeIndex")),
        key_condition_expression: Some(key_cond_expr),
        expression_attribute_names: Some(expr_attr_names),
        expression_attribute_values: Some(expr_attr_values),
        limit: Some(1),
        select: Some(String::from("SPECIFIC_ATTRIBUTES")),
        projection_expression: Some("#hashKey, #rangeKey".to_owned()),
        scan_index_forward: Some(false),
        ..Default::default()
    };

    let query = db.client.query(input);
    let data = match db.runtime.block_on(query) {
        Ok(data) => data,
        Err(e) => {
            return Err(EngineError::Manager(format!(
                "Fail to find last bot version in db: {:?}",
                e
            )))
        }
    };

    // The query returns an array of items (max 1, based on the limit param above).
    // If 0 item is returned it means that there is no open conversation, so simply return None
    let item_key = match data.items {
        None => return Ok(None),
        Some(items) if items.len() == 0 => return Ok(None),
        Some(items) => items[0].clone(),
    };

    let bot_key: BotKeys = serde_dynamodb::from_hashmap(item_key)?;
    let key = serde_dynamodb::to_hashmap(&DynamoDbKey {
        hash: bot_key.hash,
        range: bot_key.range,
    })?;

    let input = GetItemInput {
        table_name: get_table_name()?,
        key,
        ..Default::default()
    };

    let future = db.client.get_item(input);
    let res = db.runtime.block_on(future)?;

    match res.item {
        Some(val) => {
            let bot: Bot = serde_dynamodb::from_hashmap(val)?;

            let csml_bot: DynamoBot =
                match base64::engine::general_purpose::STANDARD.decode(&bot.bot) {
                    Ok(base64decoded) => {
                        match bincode::deserialize::<DynamoBotBincode>(&base64decoded[..]) {
                            Ok(bot) => bot.to_bot(),
                            Err(_) => serde_json::from_str(&bot.bot).unwrap(),
                        }
                    }
                    Err(_) => serde_json::from_str(&bot.bot).unwrap(),
                };

            let key = format!("bots/{}/versions/{}/flows.json", bot_id, bot.version_id);
            let flows = get_flows(&key, db)?;

            let key = format!("bots/{}/versions/{}/modules.json", bot_id, bot.version_id);
            let modules = get_modules(&key, db)?;

            Ok(Some(BotVersion {
                bot: csml_bot.to_bot(flows, modules),
                version_id: bot.version_id,
                engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            }))
        }
        _ => Ok(None),
    }
}

pub fn delete_bot_version(
    bot_id: &str,
    version_id: &str,
    db: &mut DynamoDbClient,
) -> Result<(), EngineError> {
    let key = format!("bots/{}/versions/{}/flows.json", bot_id, version_id);
    aws_s3::delete_object(db, &key)?;

    let key = format!("bots/{}/versions/{}/modules.json", bot_id, version_id);
    aws_s3::delete_object(db, &key)?;

    let item_key = DynamoDbKey {
        hash: Bot::get_hash(bot_id),
        range: Bot::get_range(version_id),
    };

    let input = DeleteItemInput {
        table_name: get_table_name()?,
        key: serde_dynamodb::to_hashmap(&item_key)?,
        ..Default::default()
    };

    let future = db.client.delete_item(input);
    db.runtime.block_on(future)?;

    Ok(())
}

pub fn delete_bot_versions(bot_id: &str, db: &mut DynamoDbClient) -> Result<(), EngineError> {
    let mut pagination_key = None;

    loop {
        // 25 is the Maximum operations in a single request for BatchWriteItemInput
        let data = query_bot_version(bot_id, 25, pagination_key, db)?;

        // The query returns an array of items (max 10, based on the limit param above).
        // If 0 item is returned it means that there is no open conversation, so simply return None
        // , "last_key": :
        let items = match data.items {
            None => return Ok(()),
            Some(items) if items.len() == 0 => return Ok(()),
            Some(items) => items.clone(),
        };

        let mut write_requests = vec![];
        for item in items {
            let data: BotKeys = serde_dynamodb::from_hashmap(item.to_owned())?;

            let version_id: &str = data.range.split('#').collect::<Vec<&str>>()[1];

            let key = format!("bots/{}/versions/{}/flows.json", bot_id, version_id);
            aws_s3::delete_object(db, &key)?;

            let key = format!("bots/{}/versions/{}/modules.json", bot_id, version_id);
            aws_s3::delete_object(db, &key)?;

            let key = serde_dynamodb::to_hashmap(&DynamoDbKey {
                hash: data.hash,
                range: data.range,
            })?;

            write_requests.push(WriteRequest {
                delete_request: Some(DeleteRequest { key }),
                put_request: None,
            });
        }

        let request_items = [(get_table_name()?, write_requests)]
            .iter()
            .cloned()
            .collect();

        let input = BatchWriteItemInput {
            request_items,
            ..Default::default()
        };

        execute_batch_write_query(db, input)?;

        pagination_key = data.last_evaluated_key;
        if let None = &pagination_key {
            return Ok(());
        }
    }
}

fn query_bot_info(
    bot_id: &str,
    class: &str,
    limit: i64,
    db: &mut DynamoDbClient,
    pagination_key: Option<HashMap<String, AttributeValue>>,
) -> Result<QueryOutput, EngineError> {
    let hash = format!("bot_id:{}#", bot_id);

    let expr_attr_names = [
        (String::from("#classKey"), String::from("class")),
        (String::from("#hashKey"), String::from("hash")),
    ]
    .iter()
    .cloned()
    .collect();

    let expr_attr_values = [
        (
            String::from(":classPrefix"),
            AttributeValue {
                s: Some(String::from(class)),
                ..Default::default()
            },
        ),
        (
            String::from(":hashPrefix"),
            AttributeValue {
                s: Some(hash),
                ..Default::default()
            },
        ),
    ]
    .iter()
    .cloned()
    .collect();

    // Class key = class val and begins with (hash key, hash prefix)
    // "#hashKey = :hashVal AND begins_with(#classKey, :classPrefix)".to_owned(),
    let input = QueryInput {
        table_name: get_table_name()?,
        key_condition_expression: Some(
            "#classKey = :classPrefix AND begins_with(#hashKey, :hashPrefix)".to_owned(),
        ),
        index_name: Some("ClassByClientIndex".to_owned()),
        expression_attribute_names: Some(expr_attr_names),
        expression_attribute_values: Some(expr_attr_values),
        limit: Some(limit),
        exclusive_start_key: pagination_key,
        ..Default::default()
    };

    let future = db.client.query(input);
    let data = match db.runtime.block_on(future) {
        Ok(data) => data,
        Err(e) => return Err(EngineError::Manager(format!("query_bot_info {:?}", e))),
    };

    Ok(data)
}

pub fn delete_all_bot_data(
    bot_id: &str,
    class: &str,
    db: &mut DynamoDbClient,
) -> Result<(), EngineError> {
    let mut pagination_key = None;

    loop {
        // 25 is the Maximum operations in a single request for BatchWriteItemInput
        let data = query_bot_info(bot_id, class, 25, db, pagination_key)?;

        // The query returns an array of items (max 10, based on the limit param above).
        // If 0 item is returned it means that there is no open conversation, so simply return None
        // , "last_key": :
        let items = match data.items {
            None => return Ok(()),
            Some(items) if items.len() == 0 => return Ok(()),
            Some(items) => items.clone(),
        };

        let mut write_requests = vec![];
        for item in items {
            let class: Class = serde_dynamodb::from_hashmap(item.to_owned())?;

            let key = serde_dynamodb::to_hashmap(&DynamoDbKey {
                hash: class.hash,
                range: class.range,
            })?;

            write_requests.push(WriteRequest {
                delete_request: Some(DeleteRequest { key }),
                put_request: None,
            });
        }

        let request_items = [(get_table_name()?, write_requests)]
            .iter()
            .cloned()
            .collect();

        let input = BatchWriteItemInput {
            request_items,
            ..Default::default()
        };

        execute_batch_write_query(db, input)?;

        pagination_key = data.last_evaluated_key;
        if let None = &pagination_key {
            return Ok(());
        }
    }
}
