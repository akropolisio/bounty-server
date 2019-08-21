use serde::{Serialize, Deserialize};
use super::schema::{logs, tokens, users};
use diesel::pg::types::sql_types::Jsonb;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use std::io::Write;
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};

#[derive(Debug, Queryable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct User {
	pub id: i32,
	/// Terms & Conditions - accepted
	pub terms_signed: bool,
	pub not_resident: bool,
	/// Ethereum address
	pub address: String,
	/// Amount of AKT tokens
	pub amount: i64,
}


#[derive(Debug, PartialEq, Eq, Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
	/// Terms & Conditions - accepted
	pub terms_signed: bool,
	pub not_resident: bool,
	/// Ethereum address
	pub address: &'a str,
	/// Amount of AKT tokens
	pub amount: i64,
}

#[derive(Debug, Queryable)]
#[derive(Clone)]
pub struct Token {
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expired_at: DateTime<Utc>
}


#[derive(Debug, PartialEq, Eq, Insertable)]
#[table_name = "tokens"]
pub struct NewToken<'a> {
    pub token: &'a str
}

#[derive(AsExpression, Debug, PartialEq, Eq)]
#[sql_type = "Jsonb"]
pub struct PayloadWrapper(pub HashMap<String, String>);

impl ToSql<Jsonb, Pg> for PayloadWrapper {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_all(&[1])?;
        serde_json::to_writer(out, &self.0)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[derive(Debug, PartialEq, Eq, Insertable)]
#[table_name = "logs"]
pub struct NewLog<'a> {
    pub token: &'a str,
    pub action: &'a str,
    pub payload: PayloadWrapper
}
