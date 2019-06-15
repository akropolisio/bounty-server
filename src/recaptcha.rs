use serde::{Deserializer, Deserialize};
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
pub struct RecaptchaResponse {
	pub success: bool,
	#[serde(rename = "error-codes")]
	pub error_codes: Option<HashSet<Code>>,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Code {
	MissingSecret,
	InvalidSecret,
	MissingResponse,
	InvalidResponse,
	BadRequest,
	Unknown(String),
}

use std::fmt;
impl fmt::Display for Code {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let s = match self {
			Code::MissingSecret => "missing-input-secret",
			Code::InvalidSecret => "invalid-input-secret",
			Code::MissingResponse => "missing-input-response",
			Code::InvalidResponse => "invalid-input-response",
			Code::BadRequest => "bad-request",
			Code::Unknown(code) => code,
		};
		write!(f, "{}", s)
	}
}

impl<'de> Deserialize<'de> for Code {
	fn deserialize<D>(de: D) -> Result<Self, D::Error>
		where D: Deserializer<'de> {
		let code = String::deserialize(de)?;
		Ok(match &*code {
			"missing-input-secret" => Code::MissingSecret,
			"invalid-input-secret" => Code::InvalidSecret,
			"missing-input-response" => Code::MissingResponse,
			"invalid-input-response" => Code::InvalidResponse,
			"bad-request" => Code::BadRequest,
			_ => Code::Unknown(code),
		})
	}
}


use std::net::IpAddr;
// use actix_web::http::
use url::Url;


/// Build url to recaptcha
pub fn url(key: &str, response: &str, user_ip: Option<&IpAddr>) -> Url {
	let user_ip = user_ip.map(ToString::to_string);

	let mut url = Url::parse("https://www.google.com/recaptcha/api/siteverify").unwrap();

	url.query_pairs_mut()
	   .extend_pairs(&[("secret", key), ("response", response)]);

	if let Some(user_ip) = user_ip {
		url.query_pairs_mut().append_pair("remoteip", &user_ip);
	}

	url
}


// /// Verify a recaptcha user response
// pub fn verify(key: &str, response: &str, user_ip: Option<&IpAddr>) -> Result<(), Error> {
// 	let url = self::url(key, response, user_ip);

// 	let client = Client::new();

// 	let mut response = client.get(url).send()?;
// 	let recaptcha_response = response.json::<RecaptchaResponse>()?;

// 	match (recaptcha_response.success, recaptcha_response.error_codes) {
// 		(true, _) => Ok(()),
// 		(false, Some(errors)) => Err(Error::Codes(errors)),
// 		(false, _) => Err(Error::Codes(HashSet::new()))
// 	}
// }

#[test]
fn decoding_test() {
	extern crate serde_json as json;
	use error::Code::*;

	let resp = json::from_str::<RecaptchaResponse>(
	                                               r#"{
			"success": true,
			"error-codes": ["missing-input-secret", "foo"]
		}"#,
	).unwrap();

	assert!(resp.success);
	assert!(resp.error_codes.is_some());

	let errors = resp.error_codes.unwrap();
	assert!(errors.len() == 2);
	assert!(errors.contains(&MissingSecret));
	assert!(errors.contains(&Unknown("foo".to_string())));
}
