#[macro_use]
extern crate diesel;

use std::env;
use dotenv::dotenv;
use futures::prelude::*;

use actix_cors::Cors;
use actix_web::{web, App, Responder, HttpRequest, HttpResponse, HttpServer};
use actix_web::Error;

mod db;
mod state;
mod api;
mod recaptcha;
mod token;

fn main() -> Result<(), std::io::Error> {
	dotenv().ok();
	env_logger::init();

	let (_rekey, listen_url, cors_origin, database_url) = read_env_vars();

	let _sys = actix::System::new("actix_sys");
	initialize_state(&database_url);

	let serv = HttpServer::new(move || {
		           App::new().wrap(Cors::new()
		                           .allowed_origin(&cors_origin)
		                           .allowed_origin("https://akropolis.io")
		                           .allowed_origin("https://*.akropolis.io")
		                           .allowed_methods(vec!["GET", "POST", "OPTION"])
		                           .send_wildcard()
		                           .max_age(3600))
		          .service(web::resource("/1.0/").data(web::JsonConfig::default().limit(4096))
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
		          .service(web::resource("/1.0/get").route(web::get().to_async(search_query)))

                         // create a JWT token
		          .service(web::resource("/1.0/token").route(web::post().to(create_token)))
                         // logging of a user action
		          .service(web::resource("/1.0/log").route(web::post().to(log_action)))
		          }).bind(listen_url)?;

	println!("starting");
	serv.run()?;
	println!("exitting");
	Ok(())
}

fn read_env_vars() -> (String, String, String, String) {
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

    (rekey, listen_url, cors_origin, database_url)
}

pub fn initialize_state(database_url: &str) {
	let pool_size = if cfg!(feature = "sqlite") { 1 } else { 4 };
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
			     let conn = state.get_pool().get().expect("Can not get connection");

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
				     let conn = state.get_pool().get().expect("Can not get connection");

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

fn create_token(old_token: web::Query<api::OldToken>, req: HttpRequest) -> impl Responder {
    log::debug!("req: {:?}, old_token: {:?}", req, old_token);

    use db::schema::tokens;
    use db::models::NewToken;
    use diesel::prelude::RunQueryDsl;

    let token = token::create_token();
    let state = state::State::get();
    let conn = state
        .get_pool()
        .get()
        .expect("Can not get connection");

    diesel::insert_into(tokens::table)
        .values(NewToken { token: &token })
        .execute(&conn)
        .expect("Error saving new token");

    use db::models::{NewLog, PayloadWrapper};
    use db::schema::logs;

    let headers = req
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("the header value has not visible ASCII chars").to_string()))
        .collect();

    diesel::insert_into(logs::table)
        .values(NewLog { token: &token, action: "create_new_token", payload: PayloadWrapper(headers) })
        .execute(&conn)
        .expect("Error saving new log record");

    HttpResponse::Ok()
        .json(api::Token { status: "ok".to_string(), token })
}

fn log_action(log: web::Json<api::Log>, req: HttpRequest) -> impl Responder {
    log::debug!("req: {:?}, log: {:?}", req, log);

    use db::schema::tokens::{expired_at, token, self};
    use db::models::Token;
    use diesel::prelude::{ExpressionMethods, QueryDsl, RunQueryDsl};

    let state = state::State::get();
    let conn = state
        .get_pool()
        .get()
        .expect("Can not get connection");

    let result = tokens::table
        .filter(token.eq(&log.token))
        .filter(expired_at.gt(diesel::dsl::now))
        .first::<Token>(&conn);

    match result {
        Ok(current_token) => {
            log::debug!("current token: {:?}", current_token);

            use db::models::{NewLog, PayloadWrapper};
            use db::schema::logs;

            diesel::insert_into(logs::table)
                .values(NewLog { token: &log.token, action: &log.action, payload: PayloadWrapper(log.payload.clone()) })
                .execute(&conn)
                .expect("Error saving new log record");

            HttpResponse::Ok()
                .json(api::StatusOk { status: "ok".to_string() })
        },
        Err(err) => {
            log::debug!("token not found, reason: {:?}", err);
            HttpResponse::Unauthorized()
                .json(api::StatusError { status: "error".to_string(), reason: "invalid token".to_string() })
        }
    }
}
