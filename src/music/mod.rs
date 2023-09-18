use std::path::PathBuf;

use axum::body::StreamBody;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{extract::State, routing::get};
use axum::{Form, Json};
use hyper::{header, HeaderMap, StatusCode};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use tokio_util::io::ReaderStream;
use tracing::debug;

use crate::entities::{prelude::*, *};
use crate::AppStat;

pub(crate) fn route(state: AppStat) -> axum::Router<AppStat> {
    axum::Router::new()
        .route("/listbook", get(list_book))
        .route("/listauthor", get(listauthor))
        .route("/getauthor/:author", get(get_author_by_id))
        .route("/searchauthor", get(get_authors_by_name))
        .route("/getbook/:book", get(getbook_by_id))
        .route("/searchbook", get(getbooks_by_name))
        .route("/getfile/:book/:no", get(getfile_by_id))
        .route_layer(
            tower::ServiceBuilder::new().layer(axum::middleware::from_fn_with_state(
                state,
                super::middleware::user_auth,
            )),
        )
}

#[derive(Debug, serde::Deserialize)]
enum QueryAscDesc {
    Asc,
    Desc,
}
#[derive(Debug, serde::Deserialize)]
enum QueryBy {
    Id,
    Name,
    DateAdded,
}
#[derive(Debug, serde::Deserialize)]
struct ListArgs {
    page: u64,
    page_size: u64,
}

#[derive(Debug, serde::Serialize)]
struct ListMusic {
    name: String,
    author: i32,
    chapters: i32,
}

impl From<music::Model> for ListMusic {
    fn from(music: music::Model) -> Self {
        Self {
            name: music.name,
            author: music.author_id,
            chapters: music.chapters,
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct ListResult<T> {
    total_pages: u64,
    page: u64,
    books: Vec<T>,
}
async fn list_book(
    State(state): State<AppStat>,
    Form(args): Form<ListArgs>,
) -> Json<ListResult<ListMusic>> {
    debug!("list book");
    let books = Music::find()
        .order_by_asc(music::Column::Id)
        .paginate(&state.db, args.page_size);
    let total_pages = books.num_pages().await.unwrap();
    let pages = books.fetch_page(args.page).await.unwrap();
    Json(ListResult {
        total_pages,
        page: args.page,
        books: pages.into_iter().map(Into::into).collect(),
    })
}
#[derive(Debug, serde::Serialize)]
struct ListAuthor {
    name: String,
    avatar: String,
    description: String,
}
impl From<author::Model> for ListAuthor {
    fn from(author: author::Model) -> Self {
        Self {
            name: author.name,
            avatar: author.avatar,
            description: author.description,
        }
    }
}

async fn listauthor(
    State(state): State<AppStat>,
    Form(args): Form<ListArgs>,
) -> Json<ListResult<ListAuthor>> {
    debug!("list book");
    let authors = Author::find()
        .order_by_asc(author::Column::Id)
        .paginate(&state.db, args.page_size);
    let total_pages = authors.num_pages().await.unwrap();
    let pages = authors.fetch_page(args.page).await.unwrap();
    Json(ListResult {
        total_pages,
        page: args.page,
        books: pages.into_iter().map(Into::into).collect(),
    })
}

#[derive(Debug, serde::Serialize)]
enum GetResult<T> {
    Found(T),
    NotFound(String),
}

async fn get_author_by_id(
    State(state): State<AppStat>,
    Path(id): Path<i32>,
) -> Json<GetResult<ListAuthor>> {
    debug!("get author by id:{}", id);
    let author = Author::find_by_id(id).one(&state.db).await.unwrap();
    match author {
        Some(author) => Json(GetResult::Found(author.into())),
        None => Json(GetResult::NotFound(format!("author {} not found", id))),
    }
}

async fn get_authors_by_name(
    State(state): State<AppStat>,
    Form(args): Form<SearchArgs>,
) -> Json<ListResult<ListAuthor>> {
    // search book by name
    let authors = Author::find()
        .filter(author::Column::Name.contains(args.name))
        .order_by_asc(author::Column::Id)
        .paginate(&state.db, args.page_size);
    let total_pages = authors.num_pages().await.unwrap();
    let pages = authors.fetch_page(args.page).await.unwrap();
    Json(ListResult {
        total_pages,
        page: args.page,
        books: pages.into_iter().map(Into::into).collect(),
    })
}

async fn getbook_by_id(
    State(state): State<AppStat>,
    Path(id): Path<i32>,
) -> Json<GetResult<ListMusic>> {
    debug!("get author by id:{}", id);
    let music = Music::find_by_id(id).one(&state.db).await.unwrap();
    match music {
        Some(music) => Json(GetResult::Found(music.into())),
        None => Json(GetResult::NotFound(format!("author {} not found", id))),
    }
}

#[derive(Debug, serde::Deserialize)]
struct SearchArgs {
    name: String,
    page: u64,
    page_size: u64,
}

async fn getbooks_by_name(
    State(state): State<AppStat>,
    Form(args): Form<SearchArgs>,
) -> Json<ListResult<ListMusic>> {
    // search book by name
    let books = Music::find()
        .filter(music::Column::Name.contains(args.name))
        .order_by_asc(music::Column::Id)
        .paginate(&state.db, args.page_size);
    let total_pages = books.num_pages().await.unwrap();
    let pages = books.fetch_page(args.page).await.unwrap();
    Json(ListResult {
        total_pages,
        page: args.page,
        books: pages.into_iter().map(Into::into).collect(),
    })
}
async fn getfile_by_id(
    State(state): State<AppStat>,
    Path((bookid, chapterid)): Path<(i32, i32)>,
) -> impl IntoResponse {
    let book = Music::find_by_id(bookid).one(&state.db).await.unwrap();
    match book {
        Some(book) => {
            let mp3file = PathBuf::from(format!("{}/{:04}.mp3", book.file_folder, chapterid));
            let m4afile = PathBuf::from(format!("{}/{:04}.m4a", book.file_folder, chapterid));
            // find the file, it's either folder/chapterid.mp3 or folder/chapterid.m4a
            let real_file_name = if mp3file.exists() { mp3file } else { m4afile };
            // `File` implements `AsyncRead`
            let file = match tokio::fs::File::open(&real_file_name).await {
                Ok(file) => file,
                Err(err) => {
                    return Err((
                        StatusCode::NOT_FOUND,
                        format!(
                            "File {} not found: {}",
                            format!("{}/{:4}", book.file_folder, chapterid),
                            err
                        ),
                    ))
                }
            };
            // convert the `AsyncRead` into a `Stream`
            let stream = ReaderStream::new(file);
            // convert the `Stream` into an `axum::body::HttpBody`
            let body = StreamBody::new(stream);

            let mut headers = HeaderMap::new();
            // ([
            //     (header::CONTENT_TYPE, "text/toml; charset=utf-8"),
            //     (
            //         header::CONTENT_DISPOSITION,
            //         "attachment; filename=\"Cargo.toml\"",
            //     ),
            // ]);
            let file_name = real_file_name.file_name().unwrap().to_str().unwrap();
            let disp = format!("attachment; filename=\"{}\"", file_name);
            headers.insert(header::CONTENT_TYPE, "audio/mpeg".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, disp.parse().unwrap());
            Ok((headers, body))
        }
        None => return Err((StatusCode::NOT_FOUND, format!("book {} not found", bookid))),
    }
}
