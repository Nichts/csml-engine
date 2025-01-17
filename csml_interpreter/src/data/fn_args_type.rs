use crate::data::{
    position::Position,
    primitive::{PrimitiveArray, PrimitiveObject, PrimitiveString},
    Interval, Literal,
};
use crate::error_format::*;

use std::collections::{hash_map::Iter, HashMap};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArgsType {
    Named(HashMap<String, Literal>),
    Normal(HashMap<String, Literal>),
}

impl ArgsType {
    pub fn args_to_debug(&self, interval: Interval) -> Literal {
        match self {
            Self::Named(map) | Self::Normal(map) => {
                let mut obj = HashMap::new();

                let mut args = vec![];
                let size = map.len();
                let mut index = 0;
                let mut is_secure = false;
                while index < size {
                    let lit = map[&format!("arg{}", index)].clone();
                    if lit.secure_variable {
                        is_secure = true;
                    }
                    let value =
                        PrimitiveString::get_literal(&lit.primitive.to_string(), lit.interval);
                    args.push(value);
                    index += 1;
                }

                obj.insert(
                    "args".to_owned(),
                    PrimitiveArray::get_literal(&args, interval),
                );

                let mut lit = PrimitiveObject::get_literal(&obj, interval);
                lit.secure_variable = is_secure;
                lit.set_content_type("debug");

                lit
            }
        }
    }

    pub fn args_to_log(&self) -> String {
        match self {
            Self::Named(map) | Self::Normal(map) => {
                let mut args = vec![];
                let size = map.len();
                let mut index = 0;
                while index < size {
                    let lit = map[&format!("arg{}", index)].clone();
                    if lit.secure_variable {
                        return "secure variables can not be logged".to_string();
                    }

                    let value = lit.primitive.to_string();
                    args.push(value);
                    index += 1;
                }

                args.join(", ").to_string()
            }
        }
    }

    pub fn get<'a>(&'a self, key: &str, index: usize) -> Option<&'a Literal> {
        match self {
            Self::Named(var) => {
                match (var.get(key), index) {
                    (Some(val), _) => Some(val),
                    // tmp ?
                    (None, 0) => var.get(&format!("arg{}", index)),
                    (None, _) => None,
                }
            }
            Self::Normal(var) => var.get(&format!("arg{}", index)),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Named(var) | Self::Normal(var) => var.len(),
        }
    }

    pub fn iter(&self) -> Iter<'_, String, Literal> {
        match self {
            Self::Named(var) | Self::Normal(var) => var.iter(),
        }
    }

    pub fn populate(
        &self,
        map: &mut HashMap<String, Literal>,
        vec: &[&str],
        flow_name: &str,
        interval: Interval,
    ) -> Result<(), ErrorInfo> {
        match self {
            Self::Named(var) => {
                for (key, value) in var.iter() {
                    if !vec.contains(&(key as &str)) && key != "arg0" {
                        map.insert(key.to_owned(), value.to_owned());
                    }
                }
                Ok(())
            }
            Self::Normal(var) => {
                if vec.len() < var.len() {
                    //TODO:: error msg
                    Err(gen_error_info(
                        Position::new(interval, flow_name),
                        "to many arguments".to_owned(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn populate_json_to_literal(
        &self,
        map: &mut HashMap<String, Literal>,
        vec: &[serde_json::Value],
        flow_name: &str,
        interval: Interval,
    ) -> Result<(), ErrorInfo> {
        match self {
            Self::Named(var) => {
                for (key, value) in var.iter() {
                    let contains = vec.iter().find(|obj| {
                        if let Some(map) = obj.as_object() {
                            map.contains_key(key)
                        } else {
                            false
                        }
                    });

                    if let (None, true) = (contains, key != "arg0") {
                        map.insert(key.to_owned(), value.to_owned());
                    }
                }
                Ok(())
            }
            Self::Normal(var) => {
                if vec.len() < var.len() {
                    Err(gen_error_info(
                        Position::new(interval, flow_name),
                        "to many arguments".to_owned(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}
