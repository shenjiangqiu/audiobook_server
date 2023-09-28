use std::path::Path;

use audiobook_server::{init_log, init_mysql};
use clap::Parser;
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
    audiobook_server::tools::create_new_book(
        author_name,
        new_book_name,
        Path::new(&book_dir),
        Path::new(&source_dir),
        &db,
    )
    .await
    .unwrap();
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
