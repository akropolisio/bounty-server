use serde::{Serialize, Deserialize};
use super::schema::users;


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
