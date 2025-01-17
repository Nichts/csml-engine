use actix_web::{post, web, HttpRequest, HttpResponse};
use awc::Client;
use csml_engine::data::models::RunRequest;
use csml_engine::start_conversation;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::thread;

#[derive(Debug, Serialize, Deserialize)]
struct SnsConfirmationRequest {
    #[serde(rename = "SubscribeURL")]
    subscribe_url: String,
}

async fn confirm_subscription(body: &str) -> HttpResponse {
    let val: SnsConfirmationRequest = match serde_json::from_str(body) {
        Ok(res) => res,
        Err(_) => {
            return HttpResponse::BadRequest().body("Request body can not be properly parsed")
        }
    };

    println!("SNS SubscribeURL: {}", val.subscribe_url);

    let http = Client::default();

    match http.get(val.subscribe_url.to_owned()).send().await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::BadGateway().body("Impossible to reach SubscribeURL"),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SnsMessage {
    #[serde(rename = "Message")]
    message: String,
}

async fn handle_notification(body: &str) -> HttpResponse {
    // All requests with an invalid should return a 200 code,
    // as we don't want the SNS event to be retried (same result).
    // Ideally, it should however raise an error on some logging/monitoring system
    let sns: SnsMessage = match serde_json::from_str(body) {
        Ok(res) => res,
        Err(err) => {
            eprintln!("SNS request notification parse error: {:?}", err);
            return HttpResponse::Ok().body("Request body can not be properly parsed");
        }
    };

    // sns message is itself a JSON encoded string containing the actual CSML request
    let csml_request: RunRequest = match serde_json::from_str(&sns.message) {
        Ok(res) => res,
        Err(err) => {
            eprintln!("SNS message notification parse error: {:?}", err);
            return HttpResponse::Ok().body("Request body is not a valid CSML request");
        }
    };

    // same behavior as /run requests
    let bot_opt = match csml_request.get_bot_opt() {
        Ok(bot_opt) => bot_opt,
        Err(err) => {
            eprintln!("SNS bot_opt parse error: {:?}", err);
            return HttpResponse::Ok().body("Request body is not a valid CSML request");
        }
    };
    let mut event = csml_request.event.to_owned();

    // event metadata should be an empty object by default
    event.metadata = match event.metadata {
        Value::Null => json!({}),
        val => val,
    };

    let res = thread::spawn(move || start_conversation(event, bot_opt))
        .join()
        .unwrap();

    match res {
        Ok(data) => HttpResponse::Ok().json(data),
        Err(err) => {
            eprintln!("EngineError: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/**
 * Handle CSML requests asynchronously as AWS SNS messages.
 * This endpoint must handle both SNS message handling and SNS
 * HTTP/HTTPS endpoint subscription requests.
 * No message will be sent to the endpoint until the subscription
 * has been properly confirmed.
 */
#[post("/sns")]
pub async fn handler(req: HttpRequest, body: web::Bytes) -> HttpResponse {
    let body_string = match std::str::from_utf8(&body) {
        Ok(res) => res,
        Err(_) => {
            return HttpResponse::BadRequest().body("Request body can not be properly parsed")
        }
    };

    // See AWS SNS docs for specification of how this endpoint is called for http/https notification event types:
    // https://docs.aws.amazon.com/sns/latest/dg/SendMessageToHttp.prepare.html#http-subscription-confirmation-json
    let sns_type = req.head().headers().get("x-amz-sns-message-type");

    if let Some(val) = sns_type {
        if val == "SubscriptionConfirmation" {
            return confirm_subscription(&body_string).await;
        }
        if val == "Notification" {
            return handle_notification(&body_string).await;
        }
    };

    // other scenarios inclure unsubscribe requests and invalid/non-SNS requests
    return HttpResponse::BadRequest().finish();
}
