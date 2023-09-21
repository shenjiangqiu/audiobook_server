use audiobook_server::{
    entities::{prelude::*, *},
    init_log, init_mysql,
};
use clap::Parser;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tracing::info;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    init_log();

    let Cli {
        db,
        book_dir,
        new_book_name,
        author_name,
        source_dir,
    } = Cli::parse();

    let db = init_mysql(&db).await;
    let db_book_dir = format!("{}/{}", author_name, new_book_name);
    let target_dir = format!("{}/{}/{}", book_dir, author_name, new_book_name);
    let count = audiobook_server::tools::arrange_new_folder(source_dir, target_dir).await;
    // create the book in db
    // first create the author
    let current_author = Author::find()
        .filter(author::Column::Name.eq(&author_name))
        .one(&db)
        .await
        .unwrap();
    // if it's none, insert a new one
    let author_id = match current_author {
        Some(author) => author.id,
        None => {
            let author = Author::insert(author::ActiveModel {
                name: sea_orm::ActiveValue::Set(author_name),
                avatar: sea_orm::ActiveValue::Set("".to_string()),
                description: sea_orm::ActiveValue::Set("".to_string()),

                ..Default::default()
            })
            .exec(&db)
            .await
            .unwrap();
            author.last_insert_id
        }
    };

    // insert the book
    let book = Music::insert(music::ActiveModel {
        name: sea_orm::ActiveValue::Set(new_book_name),
        author_id: sea_orm::ActiveValue::Set(author_id),
        chapters: sea_orm::ActiveValue::Set(count),
        file_folder: sea_orm::ActiveValue::Set(db_book_dir.clone()),
        ..Default::default()
    })
    .exec(&db)
    .await
    .unwrap();
    let book_id = book.last_insert_id;
    info!("book created:{}", book_id);
    info!("book dir:{}", db_book_dir);
    info!("book chapters:{}", count);
}

#[derive(Debug, Parser)]
pub struct Cli {
    /// the database url,start at "mysql://"
    #[clap(
        short,
        long,
        env = "DATABASE_URL",
        default_value = "mysql://root:qiuqiu123@10.10.0.2/music_db"
    )]
    db: String,

    /// the path store all books
    #[clap(short, long, env = "BOOKS", default_value = "./books")]
    book_dir: String,

    /// the name of the book to be created
    #[clap(short, long)]
    new_book_name: String,
    /// the name of the author of the book to be created
    #[clap(short, long)]
    author_name: String,
    /// the source dir of the book to be find
    #[clap(short, long)]
    source_dir: String,
}
