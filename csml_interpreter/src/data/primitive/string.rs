use crate::data::error_info::ErrorInfo;
use crate::data::position::Position;
use crate::data::primitive::array::PrimitiveArray;
use crate::data::primitive::boolean::PrimitiveBoolean;
use crate::data::primitive::float::PrimitiveFloat;
use crate::data::primitive::int::PrimitiveInt;
use crate::data::primitive::null::PrimitiveNull;
use crate::data::primitive::object::PrimitiveObject;
use crate::data::primitive::tools::*;
use crate::data::primitive::Right;
use crate::data::primitive::{Primitive, PrimitiveType};
use crate::data::{ast::Interval, message::Message, Data, Literal, MemoryType, MessageData, MSG};
use crate::data::{literal, literal::ContentType};
use crate::error_format::*;
use crate::interpreter::json_to_literal;
// use http::Uri;
use phf::phf_map;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::{collections::HashMap, sync::mpsc};
use url::form_urlencoded;
use url::form_urlencoded::Parse;
use url::Url;

////////////////////////////////////////////////////////////////////////////////
// DATA STRUCTURES
////////////////////////////////////////////////////////////////////////////////

type PrimitiveMethod = fn(
    string: &mut PrimitiveString,
    args: &HashMap<String, Literal>,
    additional_info: &Option<HashMap<String, Literal>>,
    interval: Interval,
    data: &mut Data,
    msg_data: &mut MessageData,
    sender: &Option<mpsc::Sender<MSG>>,
) -> Result<Literal, ErrorInfo>;

const FUNCTIONS: phf::Map<&'static str, (PrimitiveMethod, Right)> = phf_map! {
    "is_number" => (PrimitiveString::is_number as PrimitiveMethod, Right::Read),
    "is_int" => (PrimitiveString::is_int as PrimitiveMethod, Right::Read),
    "is_float" => (PrimitiveString::is_float as PrimitiveMethod, Right::Read),
    "type_of" => (PrimitiveString::type_of as PrimitiveMethod, Right::Read),
    "get_info" => (PrimitiveString::get_info as PrimitiveMethod, Right::Read),
    "is_error" => (PrimitiveString::is_error as PrimitiveMethod, Right::Read),
    "to_string" => (PrimitiveString::to_string as PrimitiveMethod, Right::Read),
    "to_json" => (PrimitiveString::to_csml_json as PrimitiveMethod, Right::Read),

    "encode_uri" => (PrimitiveString::encode_uri as PrimitiveMethod, Right::Read),
    "decode_uri" => (PrimitiveString::decode_uri as PrimitiveMethod, Right::Read),
    "encode_uri_component" => (PrimitiveString::encode_uri_component as PrimitiveMethod, Right::Read),
    "decode_uri_component" => (PrimitiveString::decode_uri_component as PrimitiveMethod, Right::Read),
    "encode_html_entities" => (PrimitiveString::encode_html_entities as PrimitiveMethod, Right::Read),
    "decode_html_entities" => (PrimitiveString::decode_html_entities as PrimitiveMethod, Right::Read),

    "is_email" => (PrimitiveString::is_email as PrimitiveMethod, Right::Read),
    "append" => (PrimitiveString::append as PrimitiveMethod, Right::Read),
    "contains" => (PrimitiveString::contains as PrimitiveMethod, Right::Read),
    "contains_regex" => (PrimitiveString::contains_regex as PrimitiveMethod, Right::Read),
    "replace_regex" => (PrimitiveString::replace_regex as PrimitiveMethod, Right::Read),
    "replace_all" => (PrimitiveString::replace_all as PrimitiveMethod, Right::Read),
    "replace" => (PrimitiveString::replace as PrimitiveMethod, Right::Read),

    "ends_with" => (PrimitiveString::ends_with as PrimitiveMethod, Right::Read),
    "ends_with_regex" => (PrimitiveString::ends_with_regex as PrimitiveMethod, Right::Read),
    "from_json" => (PrimitiveString::from_json as PrimitiveMethod, Right::Read),
    "is_empty" => (PrimitiveString::is_empty as PrimitiveMethod, Right::Read),
    "length" => (PrimitiveString::length as PrimitiveMethod, Right::Read),
    "match" => (PrimitiveString::do_match as PrimitiveMethod, Right::Read),
    "match_regex" => (PrimitiveString::do_match_regex as PrimitiveMethod, Right::Read),
    "starts_with" => (PrimitiveString::starts_with as PrimitiveMethod, Right::Read),
    "starts_with_regex" => (PrimitiveString::starts_with_regex as PrimitiveMethod, Right::Read),
    "to_lowercase" => (PrimitiveString::to_lowercase as PrimitiveMethod, Right::Read),
    "to_uppercase" => (PrimitiveString::to_uppercase as PrimitiveMethod, Right::Read),
    "capitalize" => (PrimitiveString::capitalize as PrimitiveMethod, Right::Read),
    "slice" => (PrimitiveString::slice as PrimitiveMethod, Right::Read),
    "split" => (PrimitiveString::split as PrimitiveMethod, Right::Read),

    "trim" => (PrimitiveString::trim as PrimitiveMethod, Right::Read),
    "trim_left" => (PrimitiveString::trim_left as PrimitiveMethod, Right::Read),
    "trim_right" => (PrimitiveString::trim_right as PrimitiveMethod, Right::Read),

    "abs" => (PrimitiveString::abs as PrimitiveMethod, Right::Read),
    "cos" => (PrimitiveString::cos as PrimitiveMethod, Right::Read),
    "ceil" =>(PrimitiveString::ceil as PrimitiveMethod, Right::Read),
    "floor" => (PrimitiveString::floor as PrimitiveMethod, Right::Read),
    "pow" => (PrimitiveString::pow as PrimitiveMethod, Right::Read),
    "round" => (PrimitiveString::round as PrimitiveMethod, Right::Read),
    "sin" => (PrimitiveString::sin as PrimitiveMethod, Right::Read),
    "sqrt" => (PrimitiveString::sqrt as PrimitiveMethod, Right::Read),
    "tan" => (PrimitiveString::tan as PrimitiveMethod, Right::Read),
    "to_int" => (PrimitiveString::to_int as PrimitiveMethod, Right::Read),
    "to_float" =>(PrimitiveString::to_float as PrimitiveMethod, Right::Read),
};

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveString {
    pub value: String,
}

////////////////////////////////////////////////////////////////////////////////
// PRIVATE FUNCTIONS
////////////////////////////////////////////////////////////////////////////////

fn encode_value(pairs: &Parse) -> String {
    let mut vec = vec![];

    for (index, (key, val)) in pairs.enumerate() {
        let encoded_value: String = if val.contains(',') {
            let split: Vec<&str> = val.split(',').collect();

            split
                .into_iter()
                .enumerate()
                .fold(String::new(), |mut acc, (index, val)| {
                    if index > 0 {
                        acc.push(',');
                    }

                    let encoded = urlencoding::encode(val);
                    acc.push_str(&encoded);

                    acc
                })
        } else {
            urlencoding::encode(&val).into_owned()
        };

        let and = if index == 0 { "" } else { "&" };
        let equal = if encoded_value.is_empty() { "" } else { "=" };

        vec.push(format!("{and}{}{equal}{}", key, encoded_value));
    }

    vec.concat()
}

fn decode_value(pairs: &Parse) -> String {
    let mut vec = vec![];

    for (index, (key, val)) in pairs.enumerate() {
        let encoded_value: String = if val.contains(',') {
            let split: Vec<&str> = val.split(',').collect();

            split
                .into_iter()
                .enumerate()
                .fold(String::new(), |mut acc, (index, val)| {
                    if index > 0 {
                        acc.push(',');
                    }

                    match urlencoding::decode(val) {
                        Ok(decoded) => acc.push_str(&decoded),
                        Err(_) => acc.push_str(val),
                    };

                    acc
                })
        } else {
            match urlencoding::decode(&val) {
                Ok(decoded) => decoded.into_owned(),
                Err(_) => val.into_owned(),
            }
        };

        let and = if index == 0 { "" } else { "&" };
        let equal = if encoded_value.is_empty() { "" } else { "=" };

        vec.push(format!("{and}{}{equal}{}", key, encoded_value));
    }

    vec.concat()
}

////////////////////////////////////////////////////////////////////////////////
// METHOD FUNCTIONS
////////////////////////////////////////////////////////////////////////////////

impl PrimitiveString {
    fn is_number(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "is_number() => boolean";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let result = string.value.parse::<f64>().is_ok();

        Ok(PrimitiveBoolean::get_literal(result, interval))
    }

    fn is_int(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "is_int() => boolean";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let result = string.value.parse::<i64>().is_ok();

        Ok(PrimitiveBoolean::get_literal(result, interval))
    }

    fn is_float(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "is_float() => boolean";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let result = string.value.parse::<f64>();

        match result {
            Ok(_float) if string.value.find('.').is_some() => {
                Ok(PrimitiveBoolean::get_literal(true, interval))
            }
            _ => Ok(PrimitiveBoolean::get_literal(false, interval)),
        }
    }

    fn is_email(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "is_email() => boolean";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let email_regex = Regex::new(r"^[^@]+@[^@]+\.[^@]+$").unwrap();

        let result = email_regex.is_match(&string.value);

        Ok(PrimitiveBoolean::get_literal(result, interval))
    }

    fn type_of(
        _string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "type_of() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        Ok(PrimitiveString::get_literal("string", interval))
    }

    fn get_info(
        _string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        literal::get_info(args, additional_info, interval, data)
    }

    fn is_error(
        _string: &mut PrimitiveString,
        _args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        _data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        match additional_info {
            Some(map) if map.contains_key("error") => {
                Ok(PrimitiveBoolean::get_literal(true, interval))
            }
            _ => Ok(PrimitiveBoolean::get_literal(false, interval)),
        }
    }

    fn to_string(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "to_string() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        Ok(PrimitiveString::get_literal(&string.to_string(), interval))
    }

    fn to_csml_json(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "to_json() => obj";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let value = string.to_string();

        let config = quickxml_to_serde::Config::new_with_custom_values(
            true,
            "@",
            "$text",
            quickxml_to_serde::NullValue::Ignore,
        );

        let xml: Option<serde_json::Value> =
            quickxml_to_serde::xml_string_to_json(value.clone(), &config).ok();

        let yaml: Option<serde_json::Value> = serde_yaml::from_str(&value).ok();

        match (&yaml, &xml) {
            (_, Some(json)) | (Some(json), _) => {
                json_to_literal(json, interval, &data.context.flow)
            }
            _ => Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                "Invalid format string is not a valid yaml or xml".to_string(),
            )),
        }
    }

    fn encode_uri(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "encode_uri() => String";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let vec: Vec<&str> = string.value.split('?').collect();

        let (q_separator, query, separator, fragment) = if vec.len() > 1 {
            let url = Url::parse(&string.value).unwrap();

            let query_pairs = url.query_pairs();
            let (q_separator, query) = match query_pairs.count() {
                0 => ("", "".to_owned()),
                _ => {
                    let query = encode_value(&query_pairs);
                    ("?", query)
                }
            };

            let (f_serparatorm, fragment) = match url.fragment() {
                Some(frag) => {
                    let pairs = form_urlencoded::parse(frag.as_bytes());
                    let fragment = encode_value(&pairs);

                    ("#", fragment)
                }
                None => ("", "".to_owned()),
            };

            (q_separator, query, f_serparatorm, fragment)
        } else {
            ("", "".to_owned(), "", "".to_owned())
        };

        Ok(PrimitiveString::get_literal(
            &format!("{}{q_separator}{query}{separator}{fragment}", vec[0]),
            interval,
        ))
    }

    fn decode_uri(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "encode_uri() => String";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let vec: Vec<&str> = string.value.split('?').collect();

        let (q_separator, query, separator, fragment) = if vec.len() > 1 {
            let url = Url::parse(&string.value).unwrap();

            let query_pairs = url.query_pairs();
            let (q_separator, query) = match query_pairs.count() {
                0 => ("", "".to_owned()),
                _ => {
                    let query = decode_value(&query_pairs);
                    ("?", query)
                }
            };

            let (f_serparatorm, fragment) = match url.fragment() {
                Some(frag) => {
                    let pairs = form_urlencoded::parse(frag.as_bytes());
                    let fragment = decode_value(&pairs);

                    ("#", fragment)
                }
                None => ("", "".to_owned()),
            };

            (q_separator, query, f_serparatorm, fragment)
        } else {
            ("", "".to_owned(), "", "".to_owned())
        };

        Ok(PrimitiveString::get_literal(
            &format!("{}{q_separator}{query}{separator}{fragment}", vec[0]),
            interval,
        ))
    }

    fn encode_uri_component(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "encode_uri_component() => String";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let encoded: String = urlencoding::encode(&string.value).into_owned();

        Ok(PrimitiveString::get_literal(&encoded, interval))
    }

    fn decode_uri_component(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "decode_uri_component() => String";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        match urlencoding::decode(&string.value) {
            Ok(decoded) => Ok(PrimitiveString::get_literal(&decoded, interval)),
            Err(_) => Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                "Invalid UTF8 string".to_string(),
            )),
        }
    }

    fn decode_html_entities(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "decode_html_entities() => String";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let decoded = html_escape::decode_html_entities(&string.value);

        Ok(PrimitiveString::get_literal(&decoded, interval))
    }

    fn encode_html_entities(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "encode_html_entities() => String";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let decoded = html_escape::encode_text(&string.value);

        Ok(PrimitiveString::get_literal(&decoded, interval))
    }
}

impl PrimitiveString {
    fn append(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "append(value: string) => string";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let mut result = string.value.to_owned();

        let value = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_APPEND.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_APPEND.to_owned(),
                ));
            }
        };

        result.push_str(value);

        Ok(PrimitiveString::get_literal(&result, interval))
    }

    fn contains(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "contains(value: string) => boolean";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let value = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_DO_MATCH.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_DO_MATCH.to_owned(),
                ));
            }
        };

        let result = string.value.contains(value);

        Ok(PrimitiveBoolean::get_literal(result, interval))
    }

    fn contains_regex(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "contains_regex(value: string) => boolean";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let value = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_CONTAINS_REGEX.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_CONTAINS_REGEX.to_owned(),
                ));
            }
        };

        let action = match Regex::new(value) {
            Ok(res) => res,
            Err(_) => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_CONTAINS_REGEX.to_owned(),
                ));
            }
        };

        let result = action.is_match(&string.value);

        Ok(PrimitiveBoolean::get_literal(result, interval))
    }

    fn replace(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "replace(value_to_replace: string, replace_by: string) => string";

        if args.len() != 2 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let (to_replace, replace_by) = match (args.get("arg0"), args.get("arg1")) {
            (Some(old), Some(new))
                if old.primitive.get_type() == PrimitiveType::PrimitiveString
                    && new.primitive.get_type() == PrimitiveType::PrimitiveString =>
            {
                (
                    Literal::get_value::<String>(
                        &old.primitive,
                        &data.context.flow,
                        interval,
                        ERROR_STRING_REPLACE.to_owned(),
                    )?,
                    Literal::get_value::<String>(
                        &new.primitive,
                        &data.context.flow,
                        interval,
                        ERROR_STRING_REPLACE.to_owned(),
                    )?,
                )
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_REPLACE.to_owned(),
                ));
            }
        };

        let new_string = string.value.replacen(to_replace, replace_by, 1);

        Ok(PrimitiveString::get_literal(&new_string, interval))
    }

    fn replace_all(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "replace_all(value_to_replace: string, replace_by: string) => string";

        if args.len() != 2 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let (to_replace, replace_by) = match (args.get("arg0"), args.get("arg1")) {
            (Some(old), Some(new))
                if old.primitive.get_type() == PrimitiveType::PrimitiveString
                    && new.primitive.get_type() == PrimitiveType::PrimitiveString =>
            {
                (
                    Literal::get_value::<String>(
                        &old.primitive,
                        &data.context.flow,
                        interval,
                        ERROR_STRING_REPLACE_ALL.to_owned(),
                    )?,
                    Literal::get_value::<String>(
                        &new.primitive,
                        &data.context.flow,
                        interval,
                        ERROR_STRING_REPLACE_ALL.to_owned(),
                    )?,
                )
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_REPLACE_ALL.to_owned(),
                ));
            }
        };

        let new_string = string.value.replace(to_replace, replace_by);

        Ok(PrimitiveString::get_literal(&new_string, interval))
    }

    fn replace_regex(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "replace_regex(regex: string, replace_by: string) => string";

        if args.len() != 2 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let (regex, replace_by) = match (args.get("arg0"), args.get("arg1")) {
            (Some(old), Some(new))
                if old.primitive.get_type() == PrimitiveType::PrimitiveString
                    && new.primitive.get_type() == PrimitiveType::PrimitiveString =>
            {
                (
                    Literal::get_value::<String>(
                        &old.primitive,
                        &data.context.flow,
                        interval,
                        ERROR_STRING_REPLACE_REGEX.to_owned(),
                    )?,
                    Literal::get_value::<String>(
                        &new.primitive,
                        &data.context.flow,
                        interval,
                        ERROR_STRING_REPLACE_REGEX.to_owned(),
                    )?,
                )
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_REPLACE_REGEX.to_owned(),
                ));
            }
        };

        let reg = match Regex::new(regex) {
            Ok(res) => res,
            Err(_) => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_REPLACE_REGEX.to_owned(),
                ));
            }
        };

        let new_string = reg.replace_all(&string.value, replace_by);

        Ok(PrimitiveString::get_literal(&new_string, interval))
    }

    fn ends_with(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "ends_with(value: string) => boolean";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let value = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_CONTAINS.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_END_WITH.to_owned(),
                ));
            }
        };

        let result = string.value.ends_with(value);

        Ok(PrimitiveBoolean::get_literal(result, interval))
    }

    fn ends_with_regex(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "ends_with_regex(value: string) => boolean";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let value = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_END_WITH_REGEX.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_END_WITH_REGEX.to_owned(),
                ));
            }
        };

        let action = match Regex::new(value) {
            Ok(res) => res,
            Err(_) => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_END_WITH_REGEX.to_owned(),
                ));
            }
        };

        for key in action.find_iter(&string.value) {
            if key.end() == string.value.len() {
                return Ok(PrimitiveBoolean::get_literal(true, interval));
            }
        }

        Ok(PrimitiveBoolean::get_literal(false, interval))
    }

    fn from_json(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "from_json() => object";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let object = match serde_json::from_str(&string.value) {
            Ok(result) => result,
            Err(_) => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_FROM_JSON.to_owned(),
                ));
            }
        };

        json_to_literal(&object, interval, &data.context.flow)
    }

    fn is_empty(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "is_empty() => boolean";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let result = string.value.is_empty();

        Ok(PrimitiveBoolean::get_literal(result, interval))
    }

    fn length(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "length() => int";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let result = string.value.len();

        Ok(PrimitiveInt::get_literal(result as i64, interval))
    }

    fn do_match(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "match(value: string>) => array";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let mut vector: Vec<Literal> = Vec::new();

        let value = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_DO_MATCH.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_DO_MATCH.to_owned(),
                ));
            }
        };

        for result in string.value.matches(value) {
            vector.push(PrimitiveString::get_literal(result, interval));
        }

        if vector.is_empty() {
            return Ok(PrimitiveNull::get_literal(interval));
        }

        Ok(PrimitiveArray::get_literal(&vector, interval))
    }

    fn do_match_regex(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "match_regex(value: string>) => array";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let mut s: &str = &string.value;
        let mut vector: Vec<Literal> = Vec::new();

        let value = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_MATCH_REGEX.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_MATCH_REGEX.to_owned(),
                ));
            }
        };

        let action = match Regex::new(value) {
            Ok(res) => res,
            Err(_) => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_VALID_REGEX.to_owned(),
                ));
            }
        };

        while let Some(result) = action.find(s) {
            vector.push(PrimitiveString::get_literal(
                &s[result.start()..result.end()],
                interval,
            ));
            s = &s[result.end()..];
        }

        if vector.is_empty() {
            return Ok(PrimitiveNull::get_literal(interval));
        }

        Ok(PrimitiveArray::get_literal(&vector, interval))
    }

    fn starts_with(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "starts_with(value: string) => boolean";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let value = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_START_WITH.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_START_WITH.to_owned(),
                ));
            }
        };

        let result = string.value.starts_with(value);

        Ok(PrimitiveBoolean::get_literal(result, interval))
    }

    fn starts_with_regex(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "starts_with_regex(value: string) => boolean";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let value = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_START_WITH_REGEX.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_START_WITH_REGEX.to_owned(),
                ));
            }
        };

        let action = match Regex::new(value) {
            Ok(res) => res,
            Err(_) => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_STRING_VALID_REGEX.to_owned(),
                ));
            }
        };

        if let Some(res) = action.find(&string.value) {
            if res.start() == 0 {
                return Ok(PrimitiveBoolean::get_literal(true, interval));
            }
        }

        Ok(PrimitiveBoolean::get_literal(false, interval))
    }

    fn to_lowercase(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "to_lowercase() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let s = &string.value;
        Ok(PrimitiveString::get_literal(&s.to_lowercase(), interval))
    }

    fn to_uppercase(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "to_uppercase() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let s = &string.value;
        Ok(PrimitiveString::get_literal(&s.to_uppercase(), interval))
    }

    fn capitalize(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "capitalize() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let s = &string.value;

        let mut c = s.chars();
        let string = match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        };

        Ok(PrimitiveString::get_literal(&string, interval))
    }

    fn slice(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "slice(start: Integer, end: Optional<Integer>) => string";
        let text_vec = string.value.chars().collect::<Vec<_>>();
        let len = text_vec.len();

        match args.len() {
            1 => match args.get("arg0") {
                Some(literal) => {
                    let mut int_start = Literal::get_value::<i64>(
                        &literal.primitive,
                        &data.context.flow,
                        literal.interval,
                        ERROR_SLICE_ARG_INT.to_owned(),
                    )?
                    .to_owned();

                    if int_start < 0 {
                        int_start += len as i64;
                    }

                    let start = match int_start {
                        value if value >= 0 && (value as usize) < len => value as usize,
                        _ => {
                            return Err(gen_error_info(
                                Position::new(interval, &data.context.flow),
                                ERROR_SLICE_ARG_LEN.to_owned(),
                            ))
                        }
                    };

                    let value = text_vec[start..].iter().cloned().collect::<String>();

                    Ok(PrimitiveString::get_literal(&value, interval))
                }
                _ => Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_SLICE_ARG_INT.to_owned(),
                )),
            },
            2 => match (args.get("arg0"), args.get("arg1")) {
                (Some(literal_start), Some(literal_end)) => {
                    let mut int_start = Literal::get_value::<i64>(
                        &literal_start.primitive,
                        &data.context.flow,
                        literal_start.interval,
                        ERROR_SLICE_ARG_INT.to_owned(),
                    )?
                    .to_owned();
                    let mut int_end = Literal::get_value::<i64>(
                        &literal_end.primitive,
                        &data.context.flow,
                        literal_end.interval,
                        ERROR_SLICE_ARG_INT.to_owned(),
                    )?
                    .to_owned();

                    if int_start < 0 {
                        int_start += len as i64;
                    }

                    if int_end.is_negative() {
                        int_end += len as i64;
                    }
                    if int_end < int_start {
                        return Err(gen_error_info(
                            Position::new(interval, &data.context.flow),
                            ERROR_SLICE_ARG2.to_owned(),
                        ));
                    }

                    let (start, end) = match (int_start, int_end) {
                        (start, end)
                            if int_start >= 0
                                && end >= 0
                                && (start as usize) < len
                                && (end as usize) <= len =>
                        {
                            (start as usize, end as usize)
                        }
                        _ => {
                            return Err(gen_error_info(
                                Position::new(interval, &data.context.flow),
                                ERROR_SLICE_ARG_LEN.to_owned(),
                            ))
                        }
                    };
                    let value = text_vec[start..end].iter().cloned().collect::<String>();

                    Ok(PrimitiveString::get_literal(&value, interval))
                }
                _ => Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_SLICE_ARG_INT.to_owned(),
                )),
            },
            _ => Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            )),
        }
    }

    fn split(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "string(separator: string) => array";

        if args.len() != 1 {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let separator = match args.get("arg0") {
            Some(res) if res.primitive.get_type() == PrimitiveType::PrimitiveString => {
                Literal::get_value::<String>(
                    &res.primitive,
                    &data.context.flow,
                    interval,
                    ERROR_STRING_SPLIT.to_owned(),
                )?
            }
            _ => {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_ARRAY_JOIN.to_owned(),
                ));
            }
        };

        let mut vector: Vec<Literal> = Vec::new();

        for result in string.value.split(separator) {
            vector.push(PrimitiveString::get_literal(result, interval));
        }

        Ok(PrimitiveArray::get_literal(&vector, interval))
    }

    fn trim(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "trim() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let s = &string.value;
        Ok(PrimitiveString::get_literal(s.trim(), interval))
    }

    fn trim_left(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "trim_left() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let s = &string.value;
        Ok(PrimitiveString::get_literal(s.trim_start(), interval))
    }

    fn trim_right(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "trim_right() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        let s = &string.value;
        Ok(PrimitiveString::get_literal(s.trim_end(), interval))
    }
}

// memory type can be set tu 'use' because the result of the operation will create a new literal.
impl PrimitiveString {
    fn abs(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "abs",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "abs",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "abs", ERROR_STRING_NUMERIC),
        ))
    }

    fn cos(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "cos",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "cos",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "cos", ERROR_STRING_NUMERIC),
        ))
    }

    fn ceil(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "ceil",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "ceil",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "ceil", ERROR_STRING_NUMERIC),
        ))
    }

    fn pow(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "pow",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "pow",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "pow", ERROR_STRING_NUMERIC),
        ))
    }

    fn floor(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "floor",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "floor",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "floor", ERROR_STRING_NUMERIC),
        ))
    }

    fn round(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "round",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "round",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "round", ERROR_STRING_NUMERIC),
        ))
    }

    fn sin(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "sin",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "sin",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "sin", ERROR_STRING_NUMERIC),
        ))
    }

    fn sqrt(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "sqrt",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "sqrt",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "sqrt", ERROR_STRING_NUMERIC),
        ))
    }

    fn tan(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "tan",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "tan",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "tan", ERROR_STRING_NUMERIC),
        ))
    }

    fn to_int(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "to_int",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "to_int",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "to_int", ERROR_STRING_NUMERIC),
        ))
    }

    fn to_float(
        string: &mut PrimitiveString,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<Literal, ErrorInfo> {
        if let Ok(int) = string.value.parse::<i64>() {
            let mut primitive = PrimitiveInt::new(int);

            let (literal, _right) = primitive.do_exec(
                "to_float",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }
        if let Ok(float) = string.value.parse::<f64>() {
            let mut primitive = PrimitiveFloat::new(float);

            let (literal, _right) = primitive.do_exec(
                "to_float",
                args,
                &MemoryType::Use,
                additional_info,
                interval,
                &ContentType::Primitive,
                data,
                msg_data,
                sender,
            )?;

            return Ok(literal);
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", "to_float", ERROR_STRING_NUMERIC),
        ))
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC FUNCTIONS
////////////////////////////////////////////////////////////////////////////////

impl PrimitiveString {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
        }
    }

    pub fn get_literal(string: &str, interval: Interval) -> Literal {
        let primitive = Box::new(PrimitiveString::new(string));

        Literal {
            content_type: "string".to_owned(),
            primitive,
            additional_info: None,
            secure_variable: false,
            interval,
        }
    }

    pub fn get_array_char(string: String, interval: Interval) -> Vec<Literal> {
        let array = string
            .chars()
            .map(|c| {
                let interval = interval;
                PrimitiveString::get_literal(&c.to_string(), interval)
            })
            .collect::<Vec<Literal>>();

        array
    }
}

#[typetag::serde]
impl Primitive for PrimitiveString {
    fn is_eq(&self, other: &dyn Primitive) -> bool {
        if let Some(rhs) = other.as_any().downcast_ref::<PrimitiveString>() {
            return match (get_integer(&self.value), get_integer(&rhs.value)) {
                (Ok(Integer::Int(lhs)), Ok(Integer::Float(rhs))) => (lhs as f64) == rhs,
                (Ok(Integer::Float(lhs)), Ok(Integer::Int(rhs))) => lhs == (rhs as f64),
                _ => self.value == rhs.value,
            };
        }

        false
    }

    fn is_cmp(&self, other: &dyn Primitive) -> Option<Ordering> {
        if let Some(rhs) = other.as_any().downcast_ref::<PrimitiveString>() {
            return match (get_integer(&self.value), get_integer(&rhs.value)) {
                (Ok(Integer::Int(lhs)), Ok(Integer::Float(rhs))) => (lhs as f64).partial_cmp(&rhs),
                (Ok(Integer::Float(lhs)), Ok(Integer::Int(rhs))) => lhs.partial_cmp(&(rhs as f64)),
                _ => self.value.partial_cmp(&rhs.value),
            };
        }

        None
    }

    fn do_add(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        let rhs = match other.as_any().downcast_ref::<PrimitiveString>() {
            Some(res) => res,
            None => {
                return Err(ERROR_STRING_RHS.to_owned());
            }
        };

        if let Ok(number) = get_integer(&self.value) {
            match (number, get_integer(&rhs.value)) {
                (Integer::Int(lhs), Ok(Integer::Int(rhs))) => {
                    Ok(Box::new(PrimitiveInt::new(lhs + rhs)))
                }
                (Integer::Float(lhs), Ok(Integer::Float(rhs))) => {
                    Ok(Box::new(PrimitiveFloat::new(lhs + rhs)))
                }
                (Integer::Int(lhs), Ok(Integer::Float(rhs))) => {
                    Ok(Box::new(PrimitiveFloat::new(lhs as f64 + rhs)))
                }
                (Integer::Float(lhs), Ok(Integer::Int(rhs))) => {
                    Ok(Box::new(PrimitiveFloat::new(lhs + rhs as f64)))
                }
                _ => Err(format!(
                    "{} {:?} + {:?}",
                    ERROR_ILLEGAL_OPERATION,
                    self.get_type(),
                    other.get_type()
                )),
            }
        } else {
            let mut new_string = self.value.clone();

            new_string.push_str(&rhs.value);

            Ok(Box::new(PrimitiveString::new(&new_string)))
        }
    }

    fn do_sub(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        let rhs = match other.as_any().downcast_ref::<PrimitiveString>() {
            Some(res) => res,
            None => {
                return Err(ERROR_STRING_RHS.to_owned());
            }
        };

        match (get_integer(&self.value), get_integer(&rhs.value)) {
            (Ok(Integer::Int(lhs)), Ok(Integer::Int(rhs))) => {
                Ok(Box::new(PrimitiveInt::new(lhs - rhs)))
            }
            (Ok(Integer::Float(lhs)), Ok(Integer::Float(rhs))) => {
                Ok(Box::new(PrimitiveFloat::new(lhs - rhs)))
            }
            (Ok(Integer::Int(lhs)), Ok(Integer::Float(rhs))) => {
                Ok(Box::new(PrimitiveFloat::new(lhs as f64 - rhs)))
            }
            (Ok(Integer::Float(lhs)), Ok(Integer::Int(rhs))) => {
                Ok(Box::new(PrimitiveFloat::new(lhs - rhs as f64)))
            }
            _ => Err(format!(
                "{} {:?} - {:?}",
                ERROR_ILLEGAL_OPERATION,
                self.get_type(),
                other.get_type()
            )),
        }
    }

    fn do_div(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        let rhs = match other.as_any().downcast_ref::<PrimitiveString>() {
            Some(res) => res,
            None => {
                return Err(ERROR_STRING_RHS.to_owned());
            }
        };

        match (get_integer(&self.value), get_integer(&rhs.value)) {
            (Ok(Integer::Int(lhs)), Ok(Integer::Int(rhs))) => {
                check_division_by_zero_i64(lhs, rhs)?;

                Ok(Box::new(PrimitiveInt::new(lhs / rhs)))
            }
            (Ok(Integer::Float(lhs)), Ok(Integer::Float(rhs))) => {
                check_division_by_zero_i64(lhs as i64, rhs as i64)?;

                Ok(Box::new(PrimitiveFloat::new(lhs / rhs)))
            }
            (Ok(Integer::Int(lhs)), Ok(Integer::Float(rhs))) => {
                check_division_by_zero_i64(lhs, rhs as i64)?;

                Ok(Box::new(PrimitiveFloat::new(lhs as f64 / rhs)))
            }
            (Ok(Integer::Float(lhs)), Ok(Integer::Int(rhs))) => {
                check_division_by_zero_i64(lhs as i64, rhs)?;

                Ok(Box::new(PrimitiveFloat::new(lhs / rhs as f64)))
            }
            _ => Err(format!(
                "{} {:?} / {:?}",
                ERROR_ILLEGAL_OPERATION,
                self.get_type(),
                other.get_type()
            )),
        }
    }

    fn do_mul(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        let rhs = match other.as_any().downcast_ref::<PrimitiveString>() {
            Some(res) => res,
            None => {
                return Err(ERROR_STRING_RHS.to_owned());
            }
        };

        match (get_integer(&self.value), get_integer(&rhs.value)) {
            (Ok(Integer::Int(lhs)), Ok(Integer::Int(rhs))) => {
                Ok(Box::new(PrimitiveInt::new(lhs * rhs)))
            }
            (Ok(Integer::Float(lhs)), Ok(Integer::Float(rhs))) => {
                Ok(Box::new(PrimitiveFloat::new(lhs * rhs)))
            }
            (Ok(Integer::Int(lhs)), Ok(Integer::Float(rhs))) => {
                Ok(Box::new(PrimitiveFloat::new(lhs as f64 * rhs)))
            }
            (Ok(Integer::Float(lhs)), Ok(Integer::Int(rhs))) => {
                Ok(Box::new(PrimitiveFloat::new(lhs * rhs as f64)))
            }
            _ => Err(format!(
                "{} {:?} * {:?}",
                ERROR_ILLEGAL_OPERATION,
                self.get_type(),
                other.get_type()
            )),
        }
    }

    fn do_rem(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        let rhs = match other.as_any().downcast_ref::<PrimitiveString>() {
            Some(res) => res,
            None => {
                return Err(ERROR_STRING_RHS.to_owned());
            }
        };

        match (get_integer(&self.value), get_integer(&rhs.value)) {
            (Ok(Integer::Int(lhs)), Ok(Integer::Int(rhs))) => {
                Ok(Box::new(PrimitiveInt::new(lhs % rhs)))
            }
            (Ok(Integer::Float(lhs)), Ok(Integer::Float(rhs))) => {
                Ok(Box::new(PrimitiveFloat::new(lhs % rhs)))
            }
            (Ok(Integer::Int(lhs)), Ok(Integer::Float(rhs))) => {
                Ok(Box::new(PrimitiveFloat::new(lhs as f64 % rhs)))
            }
            (Ok(Integer::Float(lhs)), Ok(Integer::Int(rhs))) => {
                Ok(Box::new(PrimitiveFloat::new(lhs * rhs as f64)))
            }
            _ => Err(format!(
                "{} {:?} % {:?}",
                ERROR_ILLEGAL_OPERATION,
                self.get_type(),
                other.get_type()
            )),
        }
    }

    fn as_debug(&self) -> &dyn std::fmt::Debug {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_type(&self) -> PrimitiveType {
        PrimitiveType::PrimitiveString
    }

    fn as_box_clone(&self) -> Box<dyn Primitive> {
        Box::new((*self).clone())
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!(self.value)
    }

    fn format_mem(&self, _content_type: &str, _first: bool) -> serde_json::Value {
        serde_json::json!(self.value)
    }

    fn to_string(&self) -> String {
        self.value.to_owned()
    }

    fn as_bool(&self) -> bool {
        true
    }

    fn get_value(&self) -> &dyn std::any::Any {
        &self.value
    }

    fn get_mut_value(&mut self) -> &mut dyn std::any::Any {
        &mut self.value
    }

    fn to_msg(&self, _content_type: String) -> Message {
        let mut hashmap: HashMap<String, Literal> = HashMap::new();

        hashmap.insert(
            "text".to_owned(),
            Literal {
                content_type: "string".to_owned(),
                primitive: Box::new(PrimitiveString::new(&self.value)),
                additional_info: None,
                secure_variable: false,
                interval: Interval {
                    start_column: 0,
                    start_line: 0,
                    offset: 0,
                    end_line: None,
                    end_column: None,
                },
            },
        );

        let mut result = PrimitiveObject::get_literal(
            &hashmap,
            Interval {
                start_column: 0,
                start_line: 0,
                offset: 0,
                end_line: None,
                end_column: None,
            },
        );
        result.set_content_type("text");

        Message {
            content_type: result.content_type,
            content: result.primitive.to_json(),
        }
    }

    fn do_exec(
        &mut self,
        name: &str,
        args: &HashMap<String, Literal>,
        mem_type: &MemoryType,
        additional_info: &Option<HashMap<String, Literal>>,
        interval: Interval,
        _content_type: &ContentType,
        data: &mut Data,
        msg_data: &mut MessageData,
        sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<(Literal, Right), ErrorInfo> {
        if let Some((f, right)) = FUNCTIONS.get(name) {
            if *mem_type == MemoryType::Constant && *right == Right::Write {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_CONSTANT_MUTABLE_FUNCTION.to_string(),
                ));
            } else {
                let res = f(
                    self,
                    args,
                    additional_info,
                    interval,
                    data,
                    msg_data,
                    sender,
                )?;

                return Ok((res, *right));
            }
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", name, ERROR_STRING_UNKNOWN_METHOD),
        ))
    }
}
