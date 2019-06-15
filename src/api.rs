use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use crate::db::models::User as DbUser;
use crate::recaptcha::Code;

type ErrorCode = u16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reg {
	pub not_resident: bool,
	/// Terms & Conditions - accepted
	pub terms: bool,
	/// Ethereum address
	pub address: String,

	pub recaptcha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Get {
	/// Ethereum address
	pub address: String,

	pub recaptcha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resp {
	pub error: Option<Error>,
	pub user: Option<User>,
}

impl From<Error> for Resp {
	fn from(error: Error) -> Self {
		Self { error: Some(error),
		       user: None }
	}
}

impl From<User> for Resp {
	fn from(user: User) -> Self {
		Self { error: None,
		       user: Some(user) }
	}
}

impl From<DbUser> for Resp {
	fn from(user: DbUser) -> Self {
		Self { error: None,
		       user: Some(user.into()) }
	}
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
	/// Ethereum address
	pub address: String,
	/// Amount of AKT tokens
	pub amount: i64,
}

impl From<crate::db::models::User> for User {
	fn from(user: DbUser) -> Self {
		Self { address: user.address.to_owned(),
		       amount: user.amount }
	}
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
	pub code: ErrorCode,
	pub message: String,
}

impl Error {
	pub fn new(code: ErrorCode) -> Self {
		Self { code: code,
		       message: match code {
			       404 => "User not found".to_owned(),
			       901 => "User should not be resident".to_owned(),
			       902 => "User have to accept Terms & Conditions".to_owned(),
			       _ => "Unknown Internal Error".to_owned(),
		       } }
	}
}


#[derive(Debug)]
pub enum ApiError {
	UserNotFound,
	UserIsResident,
	TermsNotAccepted,
	RecaptchaErr(HashSet<Code>),
}

impl ApiError {
	pub fn to_resp(self) -> Resp {
		use ApiError::*;

		match self {
			UserNotFound => Error::new(404).into(),
			UserIsResident => Error::new(901).into(),
			TermsNotAccepted => Error::new(902).into(),
			RecaptchaErr(err) => {
				Error { code: 906,
				        message: format!("{}", err.iter().fold(String::new(), |a, c| format!("{} {}", a, c))).trim().to_string() }.into()
			},
		}
	}
}


// #[derive(Debug, Serialize, Deserialize)]
// pub struct Req<T> {
// 	pub recaptcha: String,
// 	pub data: T,
// }


// pub type ReqReg = Req<Reg>;
