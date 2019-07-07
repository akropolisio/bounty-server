#![allow(stable_features)]
#![feature(async_await, await_macro, futures_api)]
#![allow(proc_macro_derive_resolution_fallback)]


#[macro_use]
extern crate diesel;

use std::env;
use dotenv::dotenv;
use futures::prelude::*;

use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web::Error;

mod db;
mod state;
mod api;
mod recaptcha;


fn main() -> Result<(), std::io::Error> {
	dotenv().ok();
	env_logger::init();

	println!("PWD: {:?}", std::env::current_dir().unwrap());

	let rekey = env::var("RECAPTCHA_KEY").expect("RECAPTCHA_KEY must be set");
	let listen_url = env::var("LISTEN_URL").expect("LISTEN_URL must be set");
	let cors_origin = env::var("CORS_ORIGIN").expect("CORS_ORIGIN must be set");

	let database_url = {
		use std::env::set_var;
		let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
		set_var("DB_READ_URL", &database_url);
		set_var("DB_WRITE_URL", &database_url);
		database_url
	};

	println!("starting with config:");
	println!("recaptcha key: {}", rekey);
	println!("listenig url: {}", listen_url);
	println!("database url: {}", database_url);
	println!("CORS origin: {}", cors_origin);

	let _sys = actix::System::new("actix_sys");
	// let state = web::Data::new(Mutex::new(dbx::db_init()));
	initialize_state(&database_url);

	let serv = HttpServer::new(move || {
		           App::new().wrap(Cors::new()
		                           .allowed_origin(&cors_origin)
		                           .allowed_origin("https://akropolis.io")
		                           .allowed_origin("https://*.akropolis.io")
		                           .allowed_methods(vec!["GET", "POST", "OPTION"])
		                           .send_wildcard()
		                           .max_age(3600))
		         //  .register_data(state.clone())
		         //  .data(web::JsonConfig::default().limit(4096))
		          .service(web::resource("/1.0/").data(web::JsonConfig::default().limit(4096))
		                                      // .route(web::get().to_async(search))
		                                         .route(web::get().to_async(search))
		                                         .route(web::post().to_async(register))
		                                         .route(web::head().to(|| HttpResponse::MethodNotAllowed())))
		          // query fallbacks:
		          .service(web::resource("/1.0/get").route(web::get().to_async(search_query)))
		          .service(web::resource("/1.0/set").route(web::get().to_async(register_query)))
		          .service(web::resource("/1.0/recaptcha_test/")
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


fn recaptcha_test(data: web::Json<api::Get>, req: HttpRequest) -> impl Future<Item = HttpResponse, Error = Error> {
	log::debug!("req: {:?}", req);
	log::debug!("data: {:?}", data);

	let fut = recaptcha_future(data.0.recaptcha, req);

	let fut = fut.map(|result| {
		             println!("result: {:?}", result);
		             match result {
			             Ok(_) => HttpResponse::NotFound().json(api::ApiError::UserNotFound.to_resp()),
		               Err(err) => HttpResponse::NotFound().json(err.to_resp()),
		             }
		            });
	fut
}


fn search_query(query: web::Query<api::Get>, req: HttpRequest) -> impl Future<Item = HttpResponse, Error = Error> {
	log::debug!("req: {:?}, query: {:?}", req, query);
	search(web::Json(query.into_inner()), req)
}

fn search(data: web::Json<api::Get>, req: HttpRequest) -> impl Future<Item = HttpResponse, Error = Error> {
	log::debug!("req: {:?}, data: {:?}", req, data);

	let fut = recaptcha_future(data.0.recaptcha.clone(), req);

	fut.map(move |result| {
		   log::debug!("recaptcha result: {:?}", result);
		   match result {
			   Ok(_) => {
			     let state = state::State::get();
			     let conn = state.get_pool().get().unwrap();

			     let user = {
				     use db::models::User;
				     use db::schema::users::dsl::{address, users};
				     use diesel::prelude::{ExpressionMethods, QueryDsl, RunQueryDsl};

				     let result = users.filter(address.eq(&data.0.address)).first::<User>(&conn);
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
			    },
		     Err(err) => HttpResponse::NotFound().json(err.to_resp()),
		   }
		  })
}


fn register_query(query: web::Query<api::Reg>, req: HttpRequest)
                  -> impl Future<Item = HttpResponse, Error = Error> {
	log::debug!("req: {:?}, query: {:?}", req, query);
	register(web::Json(query.into_inner()), req)
}

fn register(data: web::Json<api::Reg>, req: HttpRequest) -> impl Future<Item = HttpResponse, Error = Error> {
	log::debug!("req: {:?}, data: {:?}", req, data);

	let fut = recaptcha_future(data.0.recaptcha.clone(), req);

	fut.map(move |result| {
		   log::debug!("recaptcha result: {:?}", result);
		   match result {
			   Ok(_) => {
			     if !data.not_resident {
				     HttpResponse::NotFound().json(api::ApiError::UserIsResident.to_resp())
				    } else if !data.terms {
				     HttpResponse::NotFound().json(api::ApiError::TermsNotAccepted.to_resp())
				    } else {
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

					     let updated_row =
						     diesel::update(users.filter(id.eq(user.id))).set((
							not_resident.eq(data.not_resident),
							terms_signed.eq(data.terms),
						));

					     log::debug!("updating user: #{} <- {:?}", user.id, data);
					     let res = updated_row.execute(&conn);
					     log::debug!("updated user: #{} with {:?}", user.id, res);

					     HttpResponse::Found().json(api::Resp::from(user))
					    }
				    }
		     },
		     Err(err) => HttpResponse::NotFound().json(err.to_resp()),
		   }
		  })
}


fn recaptcha_future(recaptcha: String, req: HttpRequest)
                    -> impl Future<Item = Result<(), api::ApiError>, Error = Error> {
	use actix_web::client::Client;

	let client = Client::default();
	let url = {
		let key = env::var("RECAPTCHA_KEY").expect("RECAPTCHA_KEY must be set");
		let addr: Option<std::net::IpAddr> = match req.connection_info().remote().map(|addr| addr.parse()) {
			Some(Ok(addr)) => Some(addr),
			_ => None,
		};
		recaptcha::url(&key, &recaptcha, addr.as_ref())
	};

	client.get(url.as_ref())
	      .send()
	      .map(|resp| {
		      log::debug!("OK resp: {:?}", resp);
		      resp
		     })
	      .from_err()
	      .map_err(|err| {
		      log::error!("ERR resp: {:?}", err);
		      err
		     })
	      .and_then(|mut response| {
		      log::debug!("OK got resp: {:?}", response);
		      response.json::<recaptcha::RecaptchaResponse>()
		              .from_err()
		              .and_then(|response| {
			              log::debug!("OK parsed resp: {:?}", response);
			              let res = match (response.success, response.error_codes) {
				              (true, _) => Ok(()),
			                (false, Some(errors)) => Err(api::ApiError::RecaptchaErr(errors)),
			                (false, _) => Err(api::ApiError::RecaptchaErr(Default::default())),
			              };
			              futures::future::ok(res)
			             })
		              .map_err(|err| {
			              log::error!("ERR parse: {:?}", err);
			              err
			             })
		     })
}
