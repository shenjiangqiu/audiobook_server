//! get the progress of a user of some book
//!

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Form, Json, Router,
};
use hyper::StatusCode;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, EntityTrait, QueryFilter, TryIntoModel,
};
use tracing::debug;
use tracing_subscriber::field::debug;

use crate::entities::{prelude::*, *};
use crate::{middleware::LoginInfo, AppStat};
pub(crate) fn route(state: AppStat) -> Router<AppStat> {
    Router::new()
        .route("/getprogress", get(getprogress))
        .route("/setprogress", post(setprogress))
        .route_layer(
            tower::ServiceBuilder::new().layer(axum::middleware::from_fn_with_state(
                state,
                super::middleware::user_auth,
            )),
        )
}
#[derive(Debug, serde::Deserialize)]
struct FormArgs {
    book_id: i32,
}

async fn getprogress(
    State(state): State<AppStat>,
    LoginInfo {
        user_id,
        role_level,
    }: LoginInfo,
    Form(para): Form<FormArgs>,
) -> impl IntoResponse {
    debug!("getprogress: {:?},{} {}", para, user_id, role_level);
    let p = Progress::find()
        .filter(
            Condition::all()
                .add(progress::Column::AccountId.eq(user_id))
                .add(progress::Column::MusicId.eq(para.book_id)),
        )
        .one(&state.db)
        .await
        .unwrap();
    match p {
        Some(model) => (StatusCode::OK, Json(model)),
        None => {
            let model = progress::ActiveModel {
                account_id: ActiveValue::Set(user_id),
                music_id: ActiveValue::Set(para.book_id),
                chapter_no: ActiveValue::Set(0),
                progress: ActiveValue::Set(0.),
                ..Default::default()
            };
            let result = model.save(&state.db).await.unwrap();
            let model = result.try_into_model().unwrap();
            (StatusCode::OK, Json(model))
        }
    }
}
async fn setprogress(State(state): State<AppStat>, Json(modle): Json<progress::Model>) {
    debug!("setprogress: {:?}", modle);
    let chapter = modle.chapter_no;
    let progress = modle.progress;
    let mut model: progress::ActiveModel = modle.into();
    model.chapter_no = ActiveValue::Set(chapter);
    model.progress = ActiveValue::Set(progress);
    debug!("setprogress: {:?}", model);
    model.save(&state.db).await.unwrap();
}
