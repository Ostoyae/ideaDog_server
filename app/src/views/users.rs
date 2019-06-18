use crate::AppState;

use crate::midware::AuthMiddleware;
use crate::views::auth::{exist_user, perform_approve_aip};
use actix_web::http::{Method, NormalizePath, StatusCode};
use actix_web::actix::{Message, Handler};
use actix_web::{App, FutureResponse, HttpResponse, Responder, State, Path, Query};
use actix_web::{AsyncResponder, HttpRequest, Json};
use chrono::Utc;
use futures::future::{ok, Future, IntoFuture};
use ideadog::{NewUser, QueryUser, DbExecutor, Idea, QUser, QUserParams};
use serde::Deserialize;
use r2d2::Error;
use arangors::AqlQuery;
use actix_web::http::header::q;

pub fn config(cfg: App<AppState>) -> App<AppState> {
    cfg.scope("/user", |scope| {
        scope
	        .resource("/", |r| {
                r.middleware(AuthMiddleware);
                r.method(Method::GET).with(get_user);
            })
	        .default_resource(|r| {
                r.h(NormalizePath::new(
                    true,
                    true,
                    StatusCode::TEMPORARY_REDIRECT,
                ));
                r.method(Method::POST).with(create_user);
            })
	        .resource("/{id}", |r| {
		        r.method(Method::GET).with(get_user_by_id);
	        })
	        .resource("/{id}/ideas", |r|{
                r.method(Method::GET).with(get_user_ideas);
            })
        //		     .resource("/", |r| {
        //			     r.method(Method::POST).with(create_user);
        //		     })
    })
}

#[derive(Deserialize, Debug)]
pub(crate) struct SignUp {
    pub username: String,
    pub email: String,
}


#[derive(Deserialize, Debug)]
struct UIdeas(String);

fn get_user_ideas((path,state): (Path<String>, State<AppState>)) -> FutureResponse<HttpResponse> {
    state
        .database
        .send(UIdeas(path.into_inner().clone()))
        .from_err()
        .and_then( |res| match res {
            Ok(r) => Ok(HttpResponse::Ok().json(r)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        }).responder()
}

impl Message for UIdeas {
    type Result = Result<Vec<Idea>, Error>;
}

impl Handler<UIdeas> for DbExecutor {
    type Result = Result<Vec<Idea>, Error>;

    fn handle(&mut self, msg: UIdeas, ctx: &mut Self::Context) -> Self::Result {
        let conn = self.0.get().unwrap();

        let aql = AqlQuery::new(
            "FOR i in 1..1 INBOUND CONCAT('users/', @id ) idea_owner
        return i").bind_var("id", msg.0)
                  .batch_size(25);

        let response: Vec<Idea>  = match conn.aql_query(aql) {
            Ok(r) => r,
            Err(_) => vec![]
        };

        Ok(response)
    }
}


fn run_query(qufigs: QUser, state: State<AppState>) -> FutureResponse<HttpResponse> {
	state
		.database
		.send(qufigs)
		.from_err()
		.and_then(|res| match res {
			Ok(user) => Ok(HttpResponse::Ok().json(user)),
			Err(_) => Ok(HttpResponse::InternalServerError().into()),
		})
		.responder()
}

fn get_user_by_id((path, qparam, state): (Path<String>, Query<QUserParams>, State<AppState>)) -> FutureResponse<HttpResponse> {
	let qufig = QUser::ID(path.into_inner(), qparam.into_inner());

	run_query(qufig, state)
}

fn get_user((req, qparam, state): (HttpRequest<AppState>, Query<QUserParams>, State<AppState>)) -> FutureResponse<HttpResponse> {
    let tok = req
        .headers()
        .get("AUTHORIZATION")
        .map(|value| value.to_str().ok())
        .unwrap();
    //    dbg!(tok);

    let mut token = tok
        .unwrap()
        .split(" ")
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .pop()
        .unwrap();

    //    HttpResponse::Ok().finish();

//    let qufig = QueryUser { token: Some(token), id: None };
	let qufig = QUser::TOKEN(token, qparam.into_inner());

    run_query(qufig, state)
}

pub(crate) fn create_user((json, state): (Json<SignUp>, State<AppState>)) -> impl Responder {
    if exist_user(json.email.clone(), &state) {
        let response = perform_approve_aip(json.email.clone(), state);
        return response;
    };

    let new_user = NewUser {
        username: json.username.clone(),
        email: json.email.clone(),
        created_at: Utc::now().timestamp_millis(),
        active: false,
        ..NewUser::default()
    };

    let response = state
        .database
        .send(new_user)
//        .from_err()
        .and_then(|res| match res {
            Ok(ideas) => Ok(HttpResponse::Ok().json(ideas)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .wait();

    perform_approve_aip(json.email.clone(), state)
}
