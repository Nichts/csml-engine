use crate::data::AsyncConversationInfo;

async fn format_and_transfer(callback_url: &str, msg: serde_json::Value) {
    let mut request = reqwest::Client::new().post(callback_url);

    request = request
        .header("Accept", "application/json")
        .header("Content-Type", "application/json");

    let response = request.json(&msg).send().await;

    if let Err(err) = response {
        eprintln!("callback_url call failed: {:?}", err.to_string());
    }
}

/**
 * If a callback_url is defined, we must send each message to its endpoint as it comes.
 * Otherwise, just continue!
 */
pub async fn send_to_callback_url(c_info: &mut AsyncConversationInfo<'_>, msg: serde_json::Value) {
    let callback_url = match &c_info.callback_url {
        Some(callback_url) => callback_url,
        None => return,
    };

    format_and_transfer(callback_url, msg).await
}
