use csml_interpreter::data::Client;
use derive_builder::Builder;

#[derive(Builder, Debug)]
#[builder(custom_constructor, build_fn(private, name = "fallible_build"))]
pub struct ClientMessageFilter<'a> {
    pub(crate) client: &'a Client,
    #[builder(default = "25")]
    pub(crate) limit: i64,
    #[builder(setter(into), default)]
    pub(crate) pagination_key: Option<String>,
    #[builder(default)]
    pub(crate) from_date: Option<i64>,
    #[builder(default)]
    pub(crate) to_date: Option<i64>,
    #[builder(setter(into), default)]
    pub(crate) conversation_id: Option<String>,
}

impl<'a> ClientMessageFilterBuilder<'a> {
    pub fn new(client: &'a Client) -> ClientMessageFilterBuilder<'a> {
        let mut builder = Self {
            client: Some(client),
            ..Self::create_empty()
        };
        builder.client(client);
        builder
    }

    pub fn build(&self) -> ClientMessageFilter {
        self.fallible_build().expect("All required fields set at initialization")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_filter() {
        let client = Client::new("Testing".to_string(), String::default(), String::default());
        let empty_filter = ClientMessageFilterBuilder::new(&client);
        let empty_filter = empty_filter.build();

        println!("Empty Filter: {empty_filter:?}");

        assert!(matches!(empty_filter, ClientMessageFilter {
            client: &Client { ref bot_id, .. },
            limit: 25,
            ..
        } if bot_id == "Testing" ));

        let mut set_limit = ClientMessageFilterBuilder::new(&client);
        set_limit.limit(13371337);
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
