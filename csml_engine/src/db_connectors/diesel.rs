use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Text;
use diesel::{deserialize, serialize};
use std::convert::TryFrom;
use strum::{AsRefStr, EnumString, IntoStaticStr};

#[derive(
    Debug, Clone, Copy, PartialEq, AsExpression, FromSqlRow, IntoStaticStr, EnumString, AsRefStr,
)]
#[strum(serialize_all = "UPPERCASE")]
#[diesel(sql_type = Text)]
pub enum Direction {
    Send,
    Receive,
}

impl From<crate::data::models::Direction> for Direction {
    fn from(value: crate::data::models::Direction) -> Self {
        match value {
            crate::data::models::Direction::Send => Self::Send,
            crate::data::models::Direction::Receive => Self::Receive,
        }
    }
}

impl From<Direction> for crate::data::models::Direction {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Send => Self::Send,
            Direction::Receive => Self::Receive,
        }
    }
}

impl<DB> ToSql<Text, DB> for Direction
where
    DB: Backend,
    str: ToSql<Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        let string_data: &'static str = self.into();
        string_data.to_sql(out)
    }
}

impl<DB> FromSql<Text, DB> for Direction
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let string_data = String::from_sql(bytes)?;
        Ok(Self::try_from(string_data.as_str()).map_err(Box::new)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let dir = Direction::Send;
        assert_eq!(Into::<&'static str>::into(dir), "SEND");
        assert_eq!(Direction::try_from("SEND").unwrap(), dir);
    }
}
