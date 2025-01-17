#[cfg(test)]
mod tests {
    use csml_interpreter::data::{context::ContextStepInfo, CsmlBot, CsmlFlow, Message};
    use std::collections::HashMap;
    use uuid::Uuid;

    use crate::data::filter::ClientMessageFilter;
    use crate::data::models::Direction;
    use crate::{
        future::db_connectors::init_db, future::db_connectors::*, make_migrations,
        AsyncConversationInfo, Client, Context,
    };

    fn get_client() -> Client {
        Client {
            user_id: "alexis".to_owned(),
            bot_id: "botid".to_owned(),
            channel_id: "some-channel-id".to_owned(),
        }
    }

    fn get_context() -> Context {
        Context {
            current: HashMap::new(),
            metadata: HashMap::new(),
            api_info: None,
            hold: None,
            step: ContextStepInfo::Normal("start".to_owned()),
            flow: "Default".to_owned(),
            previous_bot: None,
        }
    }

    fn init_bot() -> CsmlBot {
        CsmlBot {
            id: "daee9417-8444-4ec3-8f53".to_owned(),
            name: "bot".to_owned(),
            apps_endpoint: None,
            flows: vec![CsmlFlow {
                id: "daee9417-8444-4ec3-8f53-673faff14994".to_owned(),
                name: "Default".to_owned(),
                content: "start: say \"hello\"".to_owned(),
                commands: vec![],
            }],
            native_components: None,
            custom_components: None,
            default_flow: "Default".to_owned(),
            bot_ast: None,
            no_interruption_delay: None,
            env: None,
            modules: None,
            multibot: None,
        }
    }

    fn get_conversation_info(
        messages: Vec<Message>,
        conversation_id: Uuid,
        db: AsyncDatabase,
    ) -> AsyncConversationInfo {
        AsyncConversationInfo {
            request_id: "1234".to_owned(),
            conversation_id,
            callback_url: None,
            client: get_client(),
            context: get_context(),
            metadata: serde_json::json!({}),
            messages,
            ttl: None,
            low_data: false,
            db,
        }
    }

    fn gen_message(message: &str) -> serde_json::Value {
        serde_json::json!({
            "content_type": "text",
            "content": { "text": message},
        })
    }

    #[tokio::test]
    async fn ok_bots() {
        make_migrations().unwrap_or(());

        let bot = init_bot();
        let bot_id = bot.id.clone();
        let mut db = init_db().await.unwrap();

        let bot_version = bot::create_bot_version(bot_id.clone(), bot, &mut db)
            .await
            .unwrap();

        let last_bot_version = bot::get_last_bot_version(&bot_id, &mut db)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(bot_version, last_bot_version.version_id);

        let versions = bot::get_bot_versions(&bot_id, None, None, &mut db)
            .await
            .unwrap();

        assert_eq!(bot_id, versions["bots"][0]["id"].as_str().unwrap());

        bot::delete_bot_versions(&bot_id, &mut db).await.unwrap();

        let versions = bot::get_bot_versions(&bot_id, None, None, &mut db)
            .await
            .unwrap();

        assert_eq!(0, versions["bots"].as_array().unwrap().len());
    }

    #[tokio::test]
    async fn ok_messages() {
        make_migrations().unwrap_or(());

        let client = get_client();
        let mut db = init_db().await.unwrap();
        user::delete_client(&client, &mut db).await.unwrap();

        let c_id = conversations::create_conversation("Default", "start", &client, None, &mut db)
            .await
            .unwrap();

        let msgs = vec![
            gen_message("1"),
            gen_message("2"),
            gen_message("3"),
            gen_message("4"),
        ];

        let mut data = get_conversation_info(vec![], c_id, db);

        messages::add_messages_bulk(&mut data, msgs, 0, Direction::Send)
            .await
            .unwrap();

        let filter = ClientMessageFilter::builder()
            .client(&client)
            .limit(1)
            .build();
        let mut db = init_db().await.unwrap();
        let response = messages::get_client_messages(&mut db, filter)
            .await
            .unwrap();

        let received_msgs = response.data;

        assert_eq!(1, received_msgs.len());

        assert_eq!("{\"content\": {\"text\": 4}}", received_msgs[0].payload);

        let mut db = init_db().await.unwrap();
        user::delete_client(&client, &mut db).await.unwrap();

        let filter = ClientMessageFilter::builder()
            .client(&client)
            .limit(2)
            .build();

        let mut db = init_db().await.unwrap();
        let response = messages::get_client_messages(&mut db, filter)
            .await
            .unwrap();

        let received_msgs = response.data;
        assert_eq!(0, received_msgs.len());
    }

    #[tokio::test]
    async fn ok_conversation() {
        make_migrations().unwrap_or(());

        let client = get_client();
        let mut db = init_db().await.unwrap();

        user::delete_client(&client, &mut db).await.unwrap();

        conversations::create_conversation("Default", "start", &client, None, &mut db)
            .await
            .unwrap();
        conversations::create_conversation("Default", "start", &client, None, &mut db)
            .await
            .unwrap();
        conversations::create_conversation("Default", "start", &client, None, &mut db)
            .await
            .unwrap();

        let response = conversations::get_client_conversations(&client, &mut db, Some(6), None)
            .await
            .unwrap();

        let conversations = response.data;

        assert_eq!(conversations.len(), 3);

        user::delete_client(&client, &mut db).await.unwrap();

        let response = conversations::get_client_conversations(&client, &mut db, Some(6), None)
            .await
            .unwrap();

        let conversations = response.data;
        assert_eq!(conversations.len(), 0);
    }

    #[tokio::test]
    async fn ok_memories() {
        make_migrations().unwrap_or(());

        let client = get_client();
        let mut db = init_db().await.unwrap();

        user::delete_client(&client, &mut db).await.unwrap();

        let mems = vec![
            ("key".to_owned(), serde_json::json!("value")),
            ("random".to_owned(), serde_json::json!(42)),
        ];

        for (key, value) in mems.iter() {
            memories::create_client_memory(
                &client,
                key.to_owned(),
                value.to_owned(),
                None,
                &mut db,
            )
            .await
            .unwrap();
        }

        let response = memories::internal_use_get_memories(&client, &mut db)
            .await
            .unwrap();
        let memories: &serde_json::Map<String, serde_json::Value> = response.as_object().unwrap();

        assert_eq!(memories.len(), 2);

        for (key, value) in mems.iter() {
            assert_eq!(memories.get(key).unwrap(), value);
        }

        user::delete_client(&client, &mut db).await.unwrap();

        let response = memories::internal_use_get_memories(&client, &mut db)
            .await
            .unwrap();
        let memories: &serde_json::Map<String, serde_json::Value> = response.as_object().unwrap();

        assert_eq!(memories.len(), 0);
    }

    #[tokio::test]
    async fn ok_memory() {
        make_migrations().unwrap_or(());

        let client = get_client();
        let mut db = init_db().await.unwrap();

        user::delete_client(&client, &mut db).await.unwrap();

        let mems = vec![
            ("memory_key".to_owned(), serde_json::json!("value")),
            ("memory".to_owned(), serde_json::json!("tmp")),
            ("memory_key".to_owned(), serde_json::json!("next")),
        ];

        for (key, value) in mems.iter() {
            memories::create_client_memory(
                &client,
                key.to_owned(),
                value.to_owned(),
                None,
                &mut db,
            )
            .await
            .unwrap();
        }

        let response = memories::internal_use_get_memories(&client, &mut db)
            .await
            .unwrap();
        let memories: &serde_json::Map<String, serde_json::Value> = response.as_object().unwrap();

        assert_eq!(memories.len(), 2);

        let mems = vec![
            ("memory".to_owned(), serde_json::json!("tmp")),
            ("memory_key".to_owned(), serde_json::json!("next")),
        ];

        for (key, value) in mems.iter() {
            assert_eq!(memories.get(key).unwrap(), value);
        }

        memories::delete_client_memory(&client, "memory", &mut db)
            .await
            .unwrap();

        let response = memories::internal_use_get_memories(&client, &mut db)
            .await
            .unwrap();
        let memories: &serde_json::Map<String, serde_json::Value> = response.as_object().unwrap();

        assert_eq!(memories.len(), 1);

        let mems = vec![("memory_key".to_owned(), serde_json::json!("next"))];

        for (key, value) in mems.iter() {
            assert_eq!(memories.get(key).unwrap(), value);
        }
    }

    #[tokio::test]
    async fn ok_get_memory() {
        make_migrations().unwrap_or(());

        let client = get_client();
        let mut db = init_db().await.unwrap();

        user::delete_client(&client, &mut db).await.unwrap();

        let mems = vec![
            ("my_key".to_owned(), serde_json::json!("value")),
            ("random".to_owned(), serde_json::json!("tmp")),
            ("my_key".to_owned(), serde_json::json!("next")),
        ];

        for (key, value) in mems.iter() {
            memories::create_client_memory(
                &client,
                key.to_owned(),
                value.to_owned(),
                None,
                &mut db,
            )
            .await
            .unwrap();
        }

        let response = memories::get_memory(&client, "my_key", &mut db)
            .await
            .unwrap();

        assert_eq!(
            serde_json::Value::String("next".to_owned()),
            response["value"]
        );

        let response = memories::get_memories(&client, &mut db).await.unwrap();

        match response {
            serde_json::Value::Array(memories) => {
                for memory in memories {
                    let key = memory["key"].as_str().unwrap();
                    if key != "random" && key != "my_key" {
                        panic!("bad memory => {:?}", memory)
                    }
                }
            }
            value => panic!("bad format => {:?}", value),
        }
    }
}
