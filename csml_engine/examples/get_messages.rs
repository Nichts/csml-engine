use csml_engine::get_client_messages;
use csml_interpreter::data::Client;

use chrono::prelude::*;
use csml_engine::data::filter::ClientMessageFilter;

fn main() {
    let client = Client {
        user_id: "alexis".to_owned(),
        bot_id: "botid".to_owned(),
        channel_id: "some-channel-id".to_owned(),
    };

    let filter = ClientMessageFilter::builder().client(&client).build();

    let messages = get_client_messages(filter).unwrap();

    println!("msg nbr => {}", messages.data.len());

    // DateTime::format_with_items(&self, items)
    // 2022-04-08T13:52:29.982Z
    let start = Utc.with_ymd_and_hms(2022, 4, 8, 11, 55, 50).unwrap(); // `2014-07-08T09:10:11Z`
                                                                       // dt.timestamp_millis()

    // let messages = get_client_messages(&client, None, None, None, None).unwrap();
    let filter = ClientMessageFilter::builder()
        .client(&client)
        .limit(1)
        .from_date(Some(start.timestamp()))
        .build();
    let messages = get_client_messages(filter).unwrap();

    println!("=> {:#?}", messages);

    println!("msg nbr => {}", messages.data.len());
}
