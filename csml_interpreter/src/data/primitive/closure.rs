use crate::data::error_info::ErrorInfo;
use crate::data::position::Position;
use crate::data::primitive::boolean::PrimitiveBoolean;
use crate::data::primitive::string::PrimitiveString;
use crate::data::{literal, literal::ContentType};

use crate::data::primitive::Right;
use crate::data::primitive::{Primitive, PrimitiveType};
use crate::data::{
    ast::{Expr, Interval},
    message::Message,
    Data, Literal, MemoryType, MessageData, MSG,
};
use crate::error_format::*;
use phf::phf_map;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::{collections::HashMap, sync::mpsc};

pub fn capture_variables(
    literal: &mut Literal,
    memories: HashMap<String, Literal>,
    flow_name: &str,
) {
    if literal.content_type == "closure" {
        let closure = Literal::get_mut_value::<PrimitiveClosure>(
            &mut literal.primitive,
            flow_name,
            literal.interval,
            String::new(),
        )
        .unwrap();
        closure.enclosed_variables = Some(memories);
    }
}

////////////////////////////////////////////////////////////////////////////////
// DATA STRUCTURES
////////////////////////////////////////////////////////////////////////////////

const FUNCTIONS: phf::Map<&'static str, (PrimitiveMethod, Right)> = phf_map! {
    "is_number" => (PrimitiveClosure::is_number as PrimitiveMethod, Right::Read),
    "is_int" => (PrimitiveClosure::is_int as PrimitiveMethod, Right::Read),
    "is_float" => (PrimitiveClosure::is_float as PrimitiveMethod, Right::Read),
    "type_of" => (PrimitiveClosure::type_of as PrimitiveMethod, Right::Read),
    "is_error" => (PrimitiveClosure::is_error as PrimitiveMethod, Right::Read),
    "get_info" => (PrimitiveClosure::get_info as PrimitiveMethod, Right::Read),
    "to_string" => (PrimitiveClosure::to_string as PrimitiveMethod, Right::Read),
};

type PrimitiveMethod = fn(
    int: &mut PrimitiveClosure,
    args: &HashMap<String, Literal>,
    additional_info: &Option<HashMap<String, Literal>>,
    data: &mut Data,
    interval: Interval,
) -> Result<Literal, ErrorInfo>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveClosure {
    pub args: Vec<String>,
    pub func: Box<Expr>,
    pub enclosed_variables: Option<HashMap<String, Literal>>,
}

////////////////////////////////////////////////////////////////////////////////
// METHOD FUNCTIONS
////////////////////////////////////////////////////////////////////////////////

impl PrimitiveClosure {
    fn is_number(
        _int: &mut PrimitiveClosure,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        data: &mut Data,
        interval: Interval,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "is_number() => boolean";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        Ok(PrimitiveBoolean::get_literal(false, interval))
    }

    fn is_int(
        _int: &mut PrimitiveClosure,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        data: &mut Data,
        interval: Interval,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "is_int() => boolean";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        Ok(PrimitiveBoolean::get_literal(false, interval))
    }

    fn is_float(
        _int: &mut PrimitiveClosure,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        data: &mut Data,
        interval: Interval,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "is_float() => boolean";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        Ok(PrimitiveBoolean::get_literal(false, interval))
    }

    fn type_of(
        _int: &mut PrimitiveClosure,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        data: &mut Data,
        interval: Interval,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "type_of() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        Ok(PrimitiveString::get_literal("closure", interval))
    }

    fn get_info(
        _closure: &mut PrimitiveClosure,
        args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        data: &mut Data,
        interval: Interval,
    ) -> Result<Literal, ErrorInfo> {
        literal::get_info(args, additional_info, interval, data)
    }

    fn is_error(
        _closure: &mut PrimitiveClosure,
        _args: &HashMap<String, Literal>,
        additional_info: &Option<HashMap<String, Literal>>,
        _data: &mut Data,
        interval: Interval,
    ) -> Result<Literal, ErrorInfo> {
        match additional_info {
            Some(map) if map.contains_key("error") => {
                Ok(PrimitiveBoolean::get_literal(true, interval))
            }
            _ => Ok(PrimitiveBoolean::get_literal(false, interval)),
        }
    }

    fn to_string(
        closure: &mut PrimitiveClosure,
        args: &HashMap<String, Literal>,
        _additional_info: &Option<HashMap<String, Literal>>,
        data: &mut Data,
        interval: Interval,
    ) -> Result<Literal, ErrorInfo> {
        let usage = "to_string() => string";

        if !args.is_empty() {
            return Err(gen_error_info(
                Position::new(interval, &data.context.flow),
                format!("usage: {}", usage),
            ));
        }

        Ok(PrimitiveString::get_literal(&closure.to_string(), interval))
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC FUNCTIONS
////////////////////////////////////////////////////////////////////////////////

impl PrimitiveClosure {
    pub fn new(
        args: Vec<String>,
        func: Box<Expr>,
        enclosed_variables: Option<HashMap<String, Literal>>,
    ) -> Self {
        Self {
            args,
            func,
            enclosed_variables,
        }
    }

    pub fn get_literal(
        args: Vec<String>,
        func: Box<Expr>,
        interval: Interval,
        enclosed_variables: Option<HashMap<String, Literal>>,
    ) -> Literal {
        let primitive = Box::new(PrimitiveClosure::new(args, func, enclosed_variables));

        Literal {
            content_type: "closure".to_owned(),
            primitive,
            additional_info: None,
            secure_variable: false,
            interval,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
/// TRAIT FUNCTIONS
////////////////////////////////////////////////////////////////////////////////

#[typetag::serde]
impl Primitive for PrimitiveClosure {
    fn is_eq(&self, _other: &dyn Primitive) -> bool {
        false
    }

    fn is_cmp(&self, _other: &dyn Primitive) -> Option<Ordering> {
        None
    }

    fn do_add(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        Err(format!(
            "{} {:?} + {:?}",
            ERROR_ILLEGAL_OPERATION,
            self.get_type(),
            other.get_type()
        ))
    }

    fn do_sub(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        Err(format!(
            "{} {:?} - {:?}",
            ERROR_ILLEGAL_OPERATION,
            self.get_type(),
            other.get_type()
        ))
    }

    fn do_div(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        Err(format!(
            "{} {:?} / {:?}",
            ERROR_ILLEGAL_OPERATION,
            self.get_type(),
            other.get_type()
        ))
    }

    fn do_mul(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        Err(format!(
            "{} {:?} * {:?}",
            ERROR_ILLEGAL_OPERATION,
            self.get_type(),
            other.get_type()
        ))
    }

    fn do_rem(&self, other: &dyn Primitive) -> Result<Box<dyn Primitive>, String> {
        Err(format!(
            "{} {:?} % {:?}",
            ERROR_ILLEGAL_OPERATION,
            self.get_type(),
            other.get_type()
        ))
    }

    fn as_debug(&self) -> &dyn std::fmt::Debug {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_type(&self) -> PrimitiveType {
        PrimitiveType::PrimitiveClosure
    }

    fn to_json(&self) -> serde_json::Value {
        let mut map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        map.insert("_closure".to_owned(), serde_json::json!(self));

        serde_json::Value::Object(map)
    }

    fn format_mem(&self, _content_type: &str, _first: bool) -> serde_json::Value {
        let mut map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        map.insert("_closure".to_owned(), serde_json::json!(self));

        serde_json::Value::Object(map)
    }

    fn to_string(&self) -> String {
        "Null".to_owned()
    }

    fn as_bool(&self) -> bool {
        false
    }

    fn get_value(&self) -> &dyn std::any::Any {
        self
    }

    fn get_mut_value(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_box_clone(&self) -> Box<dyn Primitive> {
        Box::new((*self).clone())
    }

    fn to_msg(&self, content_type: String) -> Message {
        Message {
            content_type,
            content: self.to_json(),
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
        _msg_data: &mut MessageData,
        _sender: &Option<mpsc::Sender<MSG>>,
    ) -> Result<(Literal, Right), ErrorInfo> {
        if let Some((f, right)) = FUNCTIONS.get(name) {
            if *mem_type == MemoryType::Constant && *right == Right::Write {
                return Err(gen_error_info(
                    Position::new(interval, &data.context.flow),
                    ERROR_CONSTANT_MUTABLE_FUNCTION.to_string(),
                ));
            } else {
                let res = f(self, args, additional_info, data, interval)?;

                return Ok((res, *right));
            }
        }

        Err(gen_error_info(
            Position::new(interval, &data.context.flow),
            format!("[{}] {}", name, ERROR_CLOSURE_UNKNOWN_METHOD),
        ))
    }
}
