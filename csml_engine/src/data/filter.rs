use csml_interpreter::data::Client;
use typed_builder::TypedBuilder;
use uuid::Uuid;

#[derive(TypedBuilder, Debug, Clone)]
pub struct ClientMessageFilter<'a> {
    pub client: &'a Client,
    #[builder(default = 25)]
    pub limit: i64,
    #[builder(setter(into), default)]
    pub pagination_key: Option<String>,
    #[builder(default)]
    pub from_date: Option<i64>,
    #[builder(default)]
    pub to_date: Option<i64>,
    #[builder(setter(into), default)]
    pub conversation_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_filter() {
        let client = Client::new("Testing".to_string(), String::default(), String::default());
        let empty_filter = ClientMessageFilter::builder().client(&client);
        let empty_filter = empty_filter.build();

        println!("Empty Filter: {empty_filter:?}");

        assert!(matches!(empty_filter, ClientMessageFilter {
            client: &Client { ref bot_id, .. },
            limit: 25,
            ..
        } if bot_id == "Testing" ));

        let set_limit = ClientMessageFilter::builder().client(&client);
        let set_limit = set_limit.limit(13371337);
        let set_limit = set_limit.build();

        println!("Set Limit Filter: {set_limit:?}");

        assert!(matches!(
            set_limit,
            ClientMessageFilter {
                limit: 13371337,
                ..
            }
        ));
    }
}
