use audiobook_server::entities::{prelude::*, *};
use audiobook_server::{init_log, init_mysql};
use clap::{Args, Parser, Subcommand, ValueEnum};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    init_log();

    let Cli { db, subcmd } = Cli::parse();
    let db = init_mysql(&db).await;
    match subcmd {
        SubCommand::Create(create_args) => {
            let CreateArgs {
                user_name,
                password,
                role,
            } = create_args;
            let password = md5::compute(password);
            let password = format!("{:x}", password);
            Account::insert(account::ActiveModel {
                name: sea_orm::ActiveValue::Set(user_name),
                password: sea_orm::ActiveValue::Set(password),
                role_level: sea_orm::ActiveValue::Set(match role {
                    Role::Admin => 0,
                    Role::User => 1,
                }),
                ..Default::default()
            })
            .exec(&db)
            .await
            .unwrap();
        }
        SubCommand::Del(del_args) => {
            let DelArgs { user_name } = del_args;
            let account = Account::find()
                .filter(account::Column::Name.eq(&user_name))
                .one(&db)
                .await
                .unwrap();
            if let Some(account) = account {
                let account = account.into_active_model();
                account.delete(&db).await.unwrap();
            }
        }
        SubCommand::Update(update_args) => {
            let UpdateArgs {
                user_name,
                new_password,
            } = update_args;
            let password = md5::compute(new_password);
            let password = format!("{:x}", password);
            let account = Account::find()
                .filter(account::Column::Name.eq(&user_name))
                .one(&db)
                .await
                .unwrap();
            if let Some(account) = account {
                let mut account = account.into_active_model();
                account.password = sea_orm::ActiveValue::Set(password);
                account.save(&db).await.unwrap();
            }
        }
        SubCommand::Migrate(migrate_args) => {
            let MigrateArgs {
                old_user_name,
                new_user_name,
                new_password,
                new_role,
            } = migrate_args;
            let mut old_account = Account::find()
                .filter(account::Column::Name.eq(&old_user_name))
                .find_with_related(Progress)
                .all(&db)
                .await
                .unwrap();
            let old_account = old_account.pop();
            if let Some(old_account) = old_account {
                let new_account = Account::insert(account::ActiveModel {
                    name: sea_orm::ActiveValue::Set(new_user_name.clone()),
                    password: sea_orm::ActiveValue::Set(format!(
                        "{:x}",
                        md5::compute(&new_password)
                    )),
                    role_level: sea_orm::ActiveValue::Set(match new_role {
                        Role::Admin => 0,
                        Role::User => 1,
                    }),
                    ..Default::default()
                })
                .exec(&db)
                .await;
                if let Ok(new_account) = new_account {
                    let new_account_id = new_account.last_insert_id;
                    // move all progress to new account
                    for p in old_account.1 {
                        let mut p = p.into_active_model();
                        p.account_id = sea_orm::ActiveValue::Set(new_account_id);
                        p.save(&db).await.unwrap();
                    }
                    old_account.0.into_active_model().delete(&db).await.unwrap();
                    println!("migrate success!")
                } else {
                    println!("fail to create new account!")
                }
            } else {
                println!("old account not found!");
            }
        }
    }
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

    #[clap(subcommand)]
    subcmd: SubCommand,
}
#[derive(Debug, Subcommand, Clone)]
enum SubCommand {
    Create(CreateArgs),
    Del(DelArgs),
    Update(UpdateArgs),
    /// delete the old one and move all related data to new one!
    Migrate(MigrateArgs),
}

#[derive(Debug, Clone, ValueEnum)]
enum Role {
    Admin,
    User,
}

#[derive(Debug, Args, Clone)]
struct CreateArgs {
    user_name: String,
    password: String,
    #[clap(default_value = "user")]
    role: Role,
}
#[derive(Debug, Args, Clone)]
struct DelArgs {
    user_name: String,
}

#[derive(Debug, Args, Clone)]
struct UpdateArgs {
    user_name: String,
    new_password: String,
}

#[derive(Debug, Args, Clone)]
struct MigrateArgs {
    old_user_name: String,
    new_user_name: String,
    new_password: String,
    #[clap(default_value = "user")]
    new_role: Role,
}
