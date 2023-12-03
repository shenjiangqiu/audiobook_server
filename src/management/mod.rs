use std::path::Path;

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Form, Json, Router,
};
use hyper::{header::LOCATION, StatusCode};
use tower::ServiceBuilder;

use crate::{tools, AppStat};

pub(crate) fn route(state: AppStat) -> Router<AppStat> {
    Router::new()
        .route("/listfile", get(listfile))
        .route("/selectpath", post(selectpath))
        .route_layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn_with_state(
                    state,
                    super::middleware::admin_auth::admin_auth,
                ))
                .layer(axum::middleware::from_fn(
                    super::middleware::log_system::log_sys,
                )),
        )
}
#[derive(Debug, serde::Serialize, Clone)]
enum FileType {
    Dir,
    File,
}
#[derive(Debug, serde::Serialize, Clone)]
struct File {
    file_type: FileType,
    file_name: String,
}
#[derive(Debug, serde::Serialize, Clone)]
struct FileList {
    code: i32,
    msg: String,
    file_list: Vec<File>,
}
#[derive(Debug, serde::Deserialize)]
struct ListFilePara {
    path: String,
}
async fn listfile(para: Form<ListFilePara>) -> impl IntoResponse {
    let mut file_list = Vec::new();
    let files = std::fs::read_dir(&para.path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(FileList {
                code: -1,
                msg: format!("read_dir error: {}", e),
                file_list: vec![],
            }),
        )
    })?;
    for file in files {
        let file = file.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(FileList {
                    code: -1,
                    msg: format!("read_dir error: {}", e),
                    file_list: vec![],
                }),
            )
        })?;
        let file_type = if file
            .file_type()
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(FileList {
                        code: -1,
                        msg: format!("read_dir error: {}", e),
                        file_list: vec![],
                    }),
                )
            })?
            .is_dir()
        {
            FileType::Dir
        } else {
            FileType::File
        };
        let file_name = file.file_name().into_string().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(FileList {
                    code: -1,
                    msg: format!("read_dir error: {:?}", e),
                    file_list: vec![],
                }),
            )
        })?;
        file_list.push(File {
            file_type,
            file_name,
        });
    }

    Ok::<_, (StatusCode, Json<FileList>)>(Json(FileList {
        code: 0,
        msg: "success".to_string(),
        file_list,
    }))
}

#[derive(Debug, serde::Deserialize)]
struct SelectPathPara {
    path: String,
    name: String,
    author: String,
}

async fn selectpath(
    State(state): State<AppStat>,
    Form(para): Form<SelectPathPara>,
) -> impl IntoResponse {
    let result = tools::create_new_book(
        para.author,
        para.name,
        &state.book_dir,
        Path::new(&para.path),
        &state.connections.db,
    )
    .await;
    match result {
        Ok(_) => (StatusCode::OK, [(LOCATION, "/")], "success"),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(LOCATION, "/")],
            "failed",
        ),
    }
}
