#![allow(stable_features)]
#![feature(async_await, await_macro, futures_api)]
#![allow(proc_macro_derive_resolution_fallback)]


#[macro_use]
extern crate diesel;

use actix_web::web::Path;
use std::env;
use std::thread;
use std::sync::Mutex;
use std::marker::PhantomData;
use std::time::Duration;

use dotenv::dotenv;

use actix_web::middleware::cors::Cors;
use actix_web::{http, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web::{http::Method, middleware};
use actix_web::client::ClientResponse;
use actix_web::client::ClientBuilder;
use failure::Error;
use failure::*;

use futures::prelude::*;
use futures::future::Future;

mod db;
mod state;
mod api;


fn main() -> Result<(), std::io::Error> {
	dotenv().ok();
	env_logger::init();

	let _rekey = env::var("RECAPTCHA_KEY").expect("RECAPTCHA_KEY must be set");
	let listen_url = env::var("LISTEN_URL").expect("LISTEN_URL must be set");
	let cors_origin = env::var("CORS_ORIGIN").expect("CORS_ORIGIN must be set");

	let database_url = {
		use std::env::set_var;
		let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
		set_var("DB_READ_URL", &database_url);
		set_var("DB_WRITE_URL", &database_url);
		database_url
	};

	let _sys = actix::System::new("actix_sys");
	// let state = web::Data::new(Mutex::new(dbx::db_init()));
	initialize_state(&database_url);

	let serv = HttpServer::new(move || {
		           App::new().wrap(Cors::new()
		                           // .disable_preflight()
											// .disable_vary_header()
		                           .allowed_origin(&cors_origin)
		                           .allowed_methods(vec!["GET", "POST"])
		                           .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
		                           .allowed_header(http::header::CONTENT_TYPE)
		                           .max_age(3600))
		         //  .register_data(state.clone())
		         //  .data(web::JsonConfig::default().limit(4096))
		          .service(web::resource("/1.0/").data(web::JsonConfig::default().limit(4096))
		                                     .route(web::get().to_async(search))
		                                     .route(web::post().to_async(register))
		                                     .route(web::head().to(|| HttpResponse::MethodNotAllowed())))
		          .service(
			          web::resource("/1.0/get").route(web::get().to_async(search_query))
					 )
		          .service(
			          web::resource("/1.0/bootstrap").route(web::get().to_async(bootstrap))
					 )

						.service(
             web::resource("/recaptcha_test/")
                 .route(web::get().to_async(recaptcha_test))
                 .route(web::post().to_async(recaptcha_test))
														 )
		          }).bind(listen_url)?;
	println!("starting");
	serv.run()?;
	println!("exitting");
	// _sys.run()
	Ok(())
}


pub fn initialize_state(database_url: &str) {
	// #[cfg(not(feature = "dbpool"))]
	// let sqldb = db::initialize();
	let pool_size = if cfg!(feature = "sqlite") { 1 } else { 4 };
	// let conn = db::establish_connection(database_url);
	let conn = db::establish_connection_pool(pool_size, database_url);
	state::State::initialize(state::State::new(conn));
}


fn recaptcha_request() -> impl Future {
	use actix_web::client::Client;

	let mut client = Client::default();

	client.get("https://www.google.com/recaptcha/api/siteverify")
	      .header("User-Agent", "Actix-web")
	      .send()
}


fn recaptcha_test(data: web::Json<api::Get>, req: HttpRequest) -> impl Future<Item = HttpResponse, Error = Error> {
	use serde::{Serialize, Deserialize};
	use actix_web::client::Client;
	log::debug!("req: {:?}", req);

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub struct RecaptchaTestAns {
		pub error: String,
	}

	let client = Client::default();

	let fut = client.get("https://www.google.com/recaptcha/api/siteverify")
	                .header("User-Agent", "Actix-web")
	                .send()
	                .map_err(|err| {
		                log::error!("resp: {:?}", err);
		                format!("{}", err)
		               })
	                .and_then(|mut response| {
		                let f = response.json::<RecaptchaTestAns>().and_then(|val| {
			                                                           println!("Response: {:?}", val);
			                                                           futures::future::ok(val)
			                                                          });
		                f.map_err(|err| {
			                 log::error!("parse: {:?}", err);
			                 format!("{}", err)
			                })
		               });

	let fut = fut.map(|answer| {
		             println!("answer: {:?}", answer);
		             HttpResponse::NotFound().json(api::ApiError::UserNotFound.to_resp())
		            });

	// TODO: ret fut
	let res = fut.wait();
	log::debug!("res: {:?}", res);

	futures::future::ok(HttpResponse::NotFound().json(api::ApiError::UserNotFound.to_resp()))
}


fn bootstrap(req: HttpRequest) -> HttpResponse {
	log::debug!("req: {:?}", req);
	use db::models::NewUser;

	let new_users = [NewUser { terms_signed: false,
	                           not_resident: false,
	                           address: "0xFOO",
	                           amount: 0 },
	                 NewUser { terms_signed: false,
	                           not_resident: false,
	                           address: "0xBAR",
	                           amount: 42 },
	                 NewUser { terms_signed: true,
	                           not_resident: true,
	                           address: "0xBOO",
	                           amount: 800 }];

	{
		use db::schema::users::dsl::*;
		use diesel::prelude::*;

		let state = state::State::get();
		let conn = state.get_pool().get().unwrap();

		// let insert = diesel::insert_into(users).values(&new_users[..]);
		// let res = insert.execute(&conn);
		for u in new_users.iter() {
			let insert = diesel::insert_into(users).values(u);
			let res = insert.execute(&conn);
			log::debug!("boot: +user: {:?}", res);
		}
	}

	HttpResponse::Ok().finish()
}

fn search_query(query: web::Query<api::Get>, req: HttpRequest) -> HttpResponse {
	log::debug!("req: {:?}, query: {:?}", req, query);
	search(web::Json(query.into_inner()), req)
}

fn search(data: web::Json<api::Get>, req: HttpRequest) -> HttpResponse {
	// req.query_string().
	log::debug!("req: {:?}, data: {:?}", req, data);

	let data: api::Get = data.0.clone();

	let state = state::State::get();
	let conn = state.get_pool().get().unwrap();

	let user = {
		use db::models::User;
		use db::schema::users::dsl::{address, users};
		use diesel::prelude::{ExpressionMethods, QueryDsl, RunQueryDsl};

		let result = users.filter(address.eq(&data.address)).first::<User>(&conn);
		match result {
			Ok(user) => {
				log::debug!("get: found: {:?}", user);
				user
			},
			Err(err) => {
				log::debug!("get: not found: {:?}", err);
				return HttpResponse::NotFound().json(api::ApiError::UserNotFound.to_resp());
			},
		}
	};

	if !user.terms_signed {
		HttpResponse::NotFound().json(api::ApiError::TermsNotAccepted.to_resp())
	} else if !user.not_resident {
		HttpResponse::NotFound().json(api::ApiError::UserIsResident.to_resp())
	} else {
		HttpResponse::Ok().json(api::Resp::from(user))
	}
}


fn register(state: web::Data<Mutex<state::State>>, data: web::Json<api::Reg>, req: HttpRequest) -> HttpResponse {
	log::debug!("req: {:?}, data: {:?}", req, data);

	let data: api::Reg = data.0.clone();

	if !data.not_resident {
		HttpResponse::NotFound().json(api::ApiError::UserIsResident.to_resp())
	} else if !data.terms {
		HttpResponse::NotFound().json(api::ApiError::TermsNotAccepted.to_resp())
	} else {
		/* {
			let key = env::var("RECAPTCHA_KEY").expect("RECAPTCHA_KEY must be set");
			let addr: Option<std::net::IpAddr> = match req.connection_info().remote().map(|addr| addr.parse()) {
				Some(Ok(addr)) => Some(addr),
				_ => None,
			};

			match recaptcha::verify(&key, &data.recaptcha, addr.as_ref()) {
				Ok(_) => {},
				Err(err) => {
					return HttpResponse::NotFound().json(api::ApiError::RecaptchaErr(err).to_resp());
				},
			}
		} */

		let state = state::State::get();
		let conn = state.get_pool().get().unwrap();

		let user = {
			use db::models::User;
			use db::schema::users::dsl::{address, users};
			use diesel::prelude::{ExpressionMethods, QueryDsl, RunQueryDsl};

			let result = users.filter(address.eq(&data.address)).first::<User>(&conn);
			match result {
				Ok(user) => {
					log::debug!("reg: found: {:?}", user);
					user
				},
				Err(err) => {
					log::debug!("reg: not found: {:?}", err);
					return HttpResponse::NotFound().json(api::ApiError::UserNotFound.to_resp());
				},
			}
		};

		{
			use diesel::prelude::*;
			use db::schema::users::dsl::{id, not_resident, terms_signed, users};

			let updated_row = diesel::update(users.filter(id.eq(user.id))).set((
				not_resident.eq(data.not_resident),
				terms_signed.eq(data.terms),
			));

			log::debug!("updating user: #{} <- {:?}", user.id, data);
			let res = updated_row.execute(&conn);
			log::debug!("updated user: #{} with {:?}", user.id, res);

			HttpResponse::Found().json(api::Resp::from(user))
		}
	}
}
