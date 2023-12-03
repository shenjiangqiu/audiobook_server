use std::env;

use crate::{
    entities::{prelude::*, *},
    progress::get_or_create_progress,
};
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Form, Router,
};
use hyper::StatusCode;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use tera::Tera;
use tracing::{error, info};

use crate::{
    middleware::{LoginInfo, PasskeyCheckResult},
    AppStat,
};

pub(crate) fn route(state: AppStat) -> Router<AppStat> {
    Router::new()
        .route("/login", get(login_page))
        .route("/logout", get(logout_page))
        .route("/index", get(index_page))
        .route("/authors", get(authors_page))
        .route("/books", get(books_page))
        .route("/book_detail", get(book_detail_page))
        .route("/author_detail", get(author_detail_page))
        .route("/player", get(player_page))
        .route("/newplayer", get(newplayer_page))
        .route("/manager", get(manager_page))
        .route("/book_manager", get(book_manager_page))
        .route("/account_manager", get(account_manager_page))
        .route("/user_op", get(user_op_page))
        .route("/update_user_page", get(update_user_page))
        .route("/create_user_action", post(create_user_action_page))
        .route("/delete_user_action", post(delete_user_action_page))
        .route("/update_user_action", post(update_user_action_page))
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(axum::middleware::from_fn_with_state(
                    state,
                    super::middleware::webui_auth::webui_auth,
                ))
                .layer(axum::middleware::from_fn(
                    super::middleware::log_system::log_sys,
                )),
        )
}
#[derive(Debug, serde::Serialize)]
struct RecentData {
    book_id: i32,
    book_name: String,
    author_id: i32,
    author: String,
    chapter_id: i32,
    progress: f64,
    progress_id: i32,
}
async fn index_html(state: &AppStat, data: &LoginInfo) -> Response {
    let tera = &state.tera;
    let mut context = tera::Context::new();
    context.insert("user_name", &data.user_name);
    context.insert("title", "sjq audiobook_server");
    let recent_played = Progress::find()
        .filter(progress::Column::AccountId.eq(data.user_id))
        .all(&state.connections.db)
        .await
        .unwrap();
    let mut recent_data = Vec::new();
    for m in recent_played {
        let book = Music::find_by_id(m.music_id)
            .one(&state.connections.db)
            .await
            .unwrap()
            .unwrap();
        let author = Author::find_by_id(book.author_id)
            .one(&state.connections.db)
            .await
            .unwrap()
            .unwrap();
        recent_data.push(RecentData {
            book_id: book.id,
            book_name: book.name,
            author: author.name,
            chapter_id: m.chapter_no,
            progress: m.progress,
            author_id: author.id,
            progress_id: m.id,
        });
    }
    context.insert("recent_played", &recent_data);
    let html = tera.render("index.tera", &context).map_err(|e| {
        error!("render error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "render error")
    });
    match html {
        Ok(html) => (StatusCode::OK, Html(html)).into_response(),
        Err(err) => err.into_response(),
    }
}

fn login_html(state: &AppStat) -> Response {
    //render login.tera
    let tera = &state.tera;
    tera.render("login.tera", &tera::Context::new())
        .map(|html| (StatusCode::OK, Html(html)))
        .map_err(|e| {
            error!("render error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "render error")
        })
        .into_response()
}
fn logout_html(state: &AppStat) -> Response {
    //render logout.tera
    info!("logout");
    let tera = &state.tera;
    tera.render("logout.tera", &tera::Context::new())
        .map(|html| (StatusCode::OK, Html(html)))
        .map_err(|e| {
            error!("render error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "render error")
        })
        .into_response()
}

async fn login_page(State(state): State<AppStat>, login_status: PasskeyCheckResult) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => index_html(&state, &data).await,
        _ => login_html(&state),
    }
}
async fn logout_page(state: State<AppStat>) -> Response {
    logout_html(&state)
}

pub(crate) async fn index_page(
    State(state): State<AppStat>,
    login_status: PasskeyCheckResult,
) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => index_html(&state, &data).await,
        _ => login_html(&state),
    }
}
#[derive(Debug, serde::Deserialize)]
struct Para {
    id: i32,
}

async fn authors_page(State(state): State<AppStat>, login_status: PasskeyCheckResult) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let authors = Author::find().all(&state.connections.db).await.unwrap();
            let mut context = tera::Context::new();
            context.insert("title", "sjq audiobook_server");
            context.insert("user_name", &data.user_name);
            context.insert("authors", &authors);
            state
                .tera
                .render("authors.tera", &context)
                .map(|html| (StatusCode::OK, Html(html)))
                .map_err(|e| {
                    error!("render error: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "render error")
                })
                .into_response()
        }
        _ => login_html(&state),
    }
}
async fn books_page(State(state): State<AppStat>, login_status: PasskeyCheckResult) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let books = Music::find().all(&state.connections.db).await.unwrap();
            let mut context = tera::Context::new();
            context.insert("title", "sjq audiobook_server");
            context.insert("user_name", &data.user_name);
            context.insert("books", &books);
            state
                .tera
                .render("books.tera", &context)
                .map(|html| (StatusCode::OK, Html(html)))
                .map_err(|e| {
                    error!("render error: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "render error")
                })
                .into_response()
        }
        _ => login_html(&state),
    }
}
async fn book_detail_page(
    State(state): State<AppStat>,
    Query(para): Query<Para>,
    login_status: PasskeyCheckResult,
) -> impl IntoResponse {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let book_id = para.id;

            let book = Music::find_by_id(book_id)
                .one(&state.connections.db)
                .await
                .unwrap()
                .unwrap();
            let author = Author::find_by_id(book.author_id)
                .one(&state.connections.db)
                .await
                .unwrap()
                .unwrap();
            let progress =
                get_or_create_progress(&state.connections.db, data.user_id, book_id).await;
            let mut context = tera::Context::new();
            // data for base
            context.insert("title", "sjq audiobook_server");
            context.insert("user_name", &data.user_name);

            context.insert("book", &book);
            context.insert("author", &author);
            context.insert("progress", &progress);
            state
                .tera
                .render("book_detail.tera", &context)
                .map(|html| (StatusCode::OK, Html(html)))
                .map_err(|e| {
                    error!("render error: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "render error")
                })
                .into_response()
        }
        _ => login_html(&state),
    }
}
async fn author_detail_page(
    State(state): State<AppStat>,
    Query(para): Query<Para>,
    login_status: PasskeyCheckResult,
) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let author_id = para.id;
            let author = Author::find_by_id(author_id)
                .one(&state.connections.db)
                .await
                .unwrap()
                .unwrap();

            let books = Music::find()
                .filter(music::Column::AuthorId.eq(author.id))
                .all(&state.connections.db)
                .await
                .unwrap();
            let mut context = tera::Context::new();
            // data for base
            context.insert("title", "sjq audiobook_server");
            context.insert("user_name", &data.user_name);

            context.insert("author_name", &author.name);
            context.insert("books", &books);
            state
                .tera
                .render("books.tera", &context)
                .map(|html| (StatusCode::OK, Html(html)))
                .map_err(|e| {
                    error!("render error: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "render error")
                })
                .into_response()
        }
        _ => login_html(&state),
    }
}

#[derive(Debug, serde::Deserialize)]
struct PlayerPara {
    book_id: i32,
    chapter_id: Option<i32>,
}

async fn player_page(
    State(state): State<AppStat>,
    Query(para): Query<PlayerPara>,
    login_status: PasskeyCheckResult,
) -> impl IntoResponse {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let book_id = para.book_id;
            let chapter_id = para.chapter_id;

            let book = Music::find_by_id(book_id)
                .one(&state.connections.db)
                .await
                .unwrap()
                .unwrap();
            let progress =
                get_or_create_progress(&state.connections.db, data.user_id, book_id).await;
            let chapter_id = chapter_id.unwrap_or(progress.chapter_no);
            let mut context = tera::Context::new();
            // data for base
            context.insert("title", &format!("Playing {}-{}", book.name, chapter_id));
            context.insert("user_name", &data.user_name);

            context.insert("user_id", &data.user_id);
            context.insert("progress", &progress);
            context.insert("book", &book);
            context.insert("chapter_id", &chapter_id);
            context.insert("chapter_id_name", &format!("{:04}", chapter_id));
            if progress.chapter_no == chapter_id {
                context.insert("this_progress", &progress.progress);
            } else {
                context.insert("this_progress", &0.);
            }

            state
                .tera
                .render("player.tera", &context)
                .map(|html| (StatusCode::OK, Html(html)))
                .map_err(|e| {
                    error!("render error: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "render error")
                })
                .into_response()
        }
        _ => login_html(&state),
    }
}

async fn newplayer_page(
    State(state): State<AppStat>,
    Query(para): Query<PlayerPara>,
    login_status: PasskeyCheckResult,
) -> impl IntoResponse {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let book_id = para.book_id;
            let chapter_id = para.chapter_id;

            let book = Music::find_by_id(book_id)
                .one(&state.connections.db)
                .await
                .unwrap()
                .unwrap();
            let progress =
                get_or_create_progress(&state.connections.db, data.user_id, book_id).await;
            let chapter_id = chapter_id.unwrap_or(progress.chapter_no);
            let mut context = tera::Context::new();
            // data for base
            context.insert("title", &format!("Playing {}-{}", book.name, chapter_id));
            context.insert("user_name", &data.user_name);

            context.insert("user_id", &data.user_id);
            context.insert("progress", &progress);
            context.insert("book", &book);
            context.insert("chapter_id", &chapter_id);
            context.insert("chapter_id_name", &format!("{:04}", chapter_id));
            if progress.chapter_no == chapter_id {
                context.insert("this_progress", &progress.progress);
            } else {
                context.insert("this_progress", &0.);
            }

            state
                .tera
                .render("newplayer.tera", &context)
                .map(|html| (StatusCode::OK, Html(html)))
                .map_err(|e| {
                    error!("render error: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "render error")
                })
                .into_response()
        }
        _ => login_html(&state),
    }
}
async fn generate_manager_page_with_data(
    data: &LoginInfo,
    tera: &Tera,
    template: &str,
    tera_context: impl IntoIterator<Item = (String, tera::Value)>,
) -> Response {
    let mut context = tera::Context::new();

    match data.role_level {
        0 => {
            context.insert("admin", &true);
        }
        1 => {
            context.insert("admin", &false);
        }
        _ => {
            panic!("no such role")
        }
    }
    // data for base
    context.insert("title", "manager");
    context.insert("user_name", &data.user_name);

    // data for template
    for (k, v) in tera_context {
        context.insert(k, &v);
    }

    tera.render(template, &context)
        .map(|html| (StatusCode::OK, Html(html)))
        .map_err(|e| {
            error!("render error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "render error")
        })
        .into_response()
}

async fn generate_manager_page(data: &LoginInfo, tera: &Tera, template: &str) -> Response {
    generate_manager_page_with_data(data, tera, template, []).await
}

async fn manager_page(
    State(state): State<AppStat>,
    login_status: PasskeyCheckResult,
) -> impl IntoResponse {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            generate_manager_page(&data, &state.tera, "manager.tera").await
        }
        _ => login_html(&state),
    }
}

async fn book_manager_page(
    State(state): State<AppStat>,
    login_status: PasskeyCheckResult,
) -> impl IntoResponse {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            generate_manager_page_with_data(
                &data,
                &state.tera,
                "book_manager.tera",
                [(
                    "current_dir".to_string(),
                    tera::to_value(env::current_dir().unwrap()).unwrap(),
                )],
            )
            .await
        }
        _ => login_html(&state),
    }
}

async fn account_manager_page(
    State(state): State<AppStat>,
    login_status: PasskeyCheckResult,
) -> impl IntoResponse {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            generate_manager_page(&data, &state.tera, "account_manager.tera").await
        }
        _ => login_html(&state),
    }
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum UserOpData {
    AddUser,
    DeleteUser,
    UpdateUser,
}

#[derive(Debug, serde::Deserialize)]
struct InputPara {
    op: UserOpData,
}

async fn user_op_page(
    State(state): State<AppStat>,
    login_status: PasskeyCheckResult,
    form: Form<InputPara>,
) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let user_list = match &form.0.op {
                UserOpData::DeleteUser | UserOpData::UpdateUser => {
                    let users = Account::find().all(&state.connections.db).await.unwrap();
                    Some(users)
                }
                _ => None,
            };
            generate_manager_page_with_data(
                &data,
                &state.tera,
                "user_op.tera",
                [
                    ("data".to_string(), tera::to_value(form.0.op).unwrap()),
                    ("user_list".to_string(), tera::to_value(user_list).unwrap()),
                ],
            )
            .await
        }
        _ => login_html(&state),
    }
}
#[derive(Debug, serde::Deserialize)]
struct UpdateUserForm {
    id: i32,
}
async fn update_user_page(
    State(state): State<AppStat>,
    login_status: PasskeyCheckResult,
    form: Form<UpdateUserForm>,
) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let user = Account::find_by_id(form.0.id)
                .one(&state.connections.db)
                .await
                .unwrap()
                .unwrap();
            let admin_selected_str = match user.role_level {
                0 => "selected='selected'",
                1 => "",
                _ => panic!("no such role"),
            };
            let user_selected_str = match user.role_level {
                0 => "",
                1 => "selected='selected'",
                _ => panic!("no such role"),
            };
            let html = format!(
                "<h1>update user</h1>
            <form action='/webui/update_user_action' method='post'>
            <lable>username</lable>
            <input type='text' name='name' value='{}'>
            <label>password</label>
            <input type='password' name='password' value=''>
            <label>role</label>
            <select name='role' value='{}'>
                <option {} value='Admin'>Admin</option>
                <option {} value='User'>User</option>
            </select>
            <input type='hidden' name='id' value='{}'>
            <button type='submit'>submit</button>
            </form>
            ",
                user.name,
                match user.role_level {
                    0 => "Admin",
                    1 => "User",
                    _ => panic!("no such role"),
                },
                admin_selected_str,
                user_selected_str,
                user.id
            );
            generate_manager_page_with_data(
                &data,
                &state.tera,
                "simple.tera",
                [("data".to_string(), tera::to_value(html).unwrap())],
            )
            .await
        }
        _ => login_html(&state),
    }
}
#[derive(Debug, serde::Deserialize)]
enum Role {
    Admin,
    User,
}
#[derive(Debug, serde::Deserialize)]
struct CreateUserForm {
    username: String,
    password: String,
    role: Role,
}
async fn create_user_action_page(
    State(state): State<AppStat>,
    login_status: PasskeyCheckResult,
    form: Form<CreateUserForm>,
) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let form = form.0;
            let password = md5::compute(form.password);
            let password = format!("{:x}", password);
            let active = account::ActiveModel {
                name: sea_orm::ActiveValue::Set(form.username),
                password: ActiveValue::Set(password),
                role_level: ActiveValue::Set(match form.role {
                    Role::Admin => 0,
                    Role::User => 1,
                }),
                ..Default::default()
            };
            let result = active.save(&state.connections.db).await;
            match result {
                Ok(_) => {
                    generate_manager_page_with_data(
                        &data,
                        &state.tera,
                        "simple.tera",
                        [(
                            "data".to_string(),
                            tera::to_value(
                                "<div><h1>success</h1></div>
                                <a href='/webui/index'>
                                    <div>
                                        <button>Home</button>
                                    </div>
                                </a>",
                            )
                            .unwrap(),
                        )],
                    )
                    .await
                }
                Err(_) => {
                    generate_manager_page_with_data(
                        &data,
                        &state.tera,
                        "simple.tera",
                        [(
                            "data".to_string(),
                            tera::to_value(
                                "<div><h1>failed, already exist</h1></div>
                        <a href='/webui/index'>
                            <div>
                                <button>Home</button>
                            </div>
                        </a>",
                            )
                            .unwrap(),
                        )],
                    )
                    .await
                }
            }
        }
        _ => login_html(&state),
    }
}

#[derive(Debug, serde::Deserialize)]
struct DeleteForm {
    id: i32,
}
async fn delete_user_action_page(
    State(state): State<AppStat>,
    login_status: PasskeyCheckResult,
    form: Form<DeleteForm>,
) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let form = form.0;

            let result = Account::delete_by_id(form.id)
                .exec(&state.connections.db)
                .await;
            match result {
                Ok(_) => {
                    generate_manager_page_with_data(
                        &data,
                        &state.tera,
                        "simple.tera",
                        [(
                            "data".to_string(),
                            tera::to_value(
                                "<div><h1>success</h1></div>
                                <a href='/webui/index'>
                                    <div>
                                        <button>Home</button>
                                    </div>
                                </a>",
                            )
                            .unwrap(),
                        )],
                    )
                    .await
                }
                Err(_) => {
                    generate_manager_page_with_data(
                        &data,
                        &state.tera,
                        "simple.tera",
                        [(
                            "data".to_string(),
                            tera::to_value(
                                "<div><h1>failed</h1></div>
                        <a href='/webui/index'>
                            <div>
                                <button>Home</button>
                            </div>
                        </a>",
                            )
                            .unwrap(),
                        )],
                    )
                    .await
                }
            }
        }
        _ => login_html(&state),
    }
}

#[derive(Debug, serde::Deserialize)]
struct UpdateActionForm {
    id: i32,
    name: String,
    password: String,
    role: Role,
}
async fn update_user_action_page(
    State(state): State<AppStat>,
    login_status: PasskeyCheckResult,
    form: Form<UpdateActionForm>,
) -> Response {
    match login_status {
        PasskeyCheckResult::LogInSucceed((_, data)) => {
            let form = form.0;
            let password = md5::compute(form.password);
            let password = format!("{:x}", password);
            let active = account::ActiveModel {
                id: ActiveValue::Unchanged(form.id),
                name: sea_orm::ActiveValue::Set(form.name),
                password: ActiveValue::Set(password),
                role_level: ActiveValue::Set(match form.role {
                    Role::Admin => 0,
                    Role::User => 1,
                }),
            };
            let result = active.save(&state.connections.db).await;

            match result {
                Ok(_) => {
                    generate_manager_page_with_data(
                        &data,
                        &state.tera,
                        "simple.tera",
                        [(
                            "data".to_string(),
                            tera::to_value(
                                "<div><h1>success</h1></div>
                                <a href='/webui/index'>
                                    <div>
                                        <button>Home</button>
                                    </div>
                                </a>",
                            )
                            .unwrap(),
                        )],
                    )
                    .await
                }
                Err(_) => {
                    generate_manager_page_with_data(
                        &data,
                        &state.tera,
                        "simple.tera",
                        [(
                            "data".to_string(),
                            tera::to_value(
                                "<div><h1>failed</h1></div>
                        <a href='/webui/index'>
                            <div>
                                <button>Home</button>
                            </div>
                        </a>",
                            )
                            .unwrap(),
                        )],
                    )
                    .await
                }
            }
        }
        _ => login_html(&state),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_name_translate() {
        let name = 12;
        let name = format!("{:04}", name);
        assert_eq!(name, "0012");
    }

    #[derive(serde::Serialize)]
    struct O {
        a: Option<A>,
    }
    #[derive(serde::Serialize)]
    enum A {
        B { a: i32 },
        C(i32),
        D,
    }
    #[test]
    fn test_json() {
        let a = A::B { a: 1 };
        let b = A::C(30);
        println!("{}", serde_json::to_string(&a).unwrap());
        println!("{}", serde_json::to_string(&b).unwrap());
        println!("{}", serde_json::to_string(&A::D).unwrap());
        println!("{}", serde_json::to_string(&O { a: Some(a) }).unwrap());
        println!("{}", serde_json::to_string(&O { a: None }).unwrap());
    }
}
