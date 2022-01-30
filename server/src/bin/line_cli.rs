use clap::{AppSettings, Arg, ArgMatches};
use reqwest::Client;

use line::http;
use server::line;
use server::line::api::LineApi;
use server::line::menu::RichMenu;

fn get_line_token(x: Option<String>) -> String {
    x.or(std::env::var("LINE_TOKEN").ok())
        .or(std::fs::read_to_string("server/src/line.token").ok())
        .expect("Please specify a line token")
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let line_token_arg = Arg::new("line-token").short('t').takes_value(true);
    let matches = clap::App::new("line")
        .version("1.0.0")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            clap::App::new("menu")
                .about("Manage line menus")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(menus::List::app().arg(line_token_arg.clone()))
                .subcommand(menus::Default::app().arg(line_token_arg.clone()))
                .subcommand(menus::SetDefault::app().arg(line_token_arg.clone()))
                .subcommand(menus::Create::app().arg(line_token_arg.clone()))
                .subcommand(menus::Delete::app().arg(line_token_arg.clone())),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("menu", cmd_matches)) => match cmd_matches.subcommand() {
            Some(("list", m)) => {
                let result = get_http_client(m).get_rich_menus().await.unwrap();
                println!("{:?}", result)
            }
            Some(("create", m)) => {
                let json_path = m.value_of("json").unwrap();
                let image_path = m.value_of("image").unwrap();
                let content = std::fs::read_to_string(json_path).unwrap();
                let menu = serde_json::from_str::<RichMenu>(&content).unwrap();
                let image = std::fs::read(image_path).unwrap();
                let client = get_http_client(m);
                let result = client.create_rich_menu(&menu, image).await.unwrap();
                println!("Created menu {:?}", result);
            }
            Some(("default", m)) => {
                let result = get_http_client(m)
                    .get_default_menu(m.value_of("id"))
                    .await
                    .unwrap();
                println!("{}", result)
            }
            Some(("delete", m)) => {
                let menu_id = m.value_of("id").unwrap();
                get_http_client(m).delete_rich_menu(menu_id).await.unwrap();
                println!("Menu {} deleted", menu_id)
            }
            Some(("default-set", m)) => {
                let menu_id = m.value_of("menu").unwrap();
                let user_id = m.value_of("id");
                get_http_client(m)
                    .set_rich_menu(menu_id, user_id)
                    .await
                    .unwrap();
                println!(
                    "Menu {} set for {}",
                    menu_id,
                    user_id
                        .map(|user_id| format!("user {}", user_id))
                        .unwrap_or("default".to_string())
                )
            }

            _ => {}
        },
        _ => {}
    }
}

fn get_http_client(m: &ArgMatches) -> Client {
    let token = get_line_token(m.value_of_t("line-token").ok());
    let result = http::get_line_client(Some(token));
    result
}

mod menus {
    use clap::{App, Arg};

    pub struct List;

    pub struct Delete;

    pub struct Create;

    pub struct Default;

    pub struct SetDefault;

    impl List {
        pub fn app() -> App<'static> {
            clap::App::new("list").about("List line menus")
        }
    }

    impl Delete {
        pub fn app() -> App<'static> {
            clap::App::new("delete")
                .about("Delete line menu")
                .arg(Arg::new("id").short('i').required(true).takes_value(true))
        }
    }

    impl Create {
        pub fn app() -> App<'static> {
            clap::App::new("create")
                .about("Create line menu")
                .arg(
                    Arg::new("json")
                        .help("Path to the source json file")
                        .short('j')
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::new("image")
                        .help("Path to the source image file")
                        .short('i')
                        .required(true)
                        .takes_value(true),
                )
        }
    }

    impl Default {
        pub fn app() -> App<'static> {
            clap::App::new("default").about("Get the default menu").arg(
                Arg::new("id")
                    .short('i')
                    .required(false)
                    .takes_value(true)
                    .help("Optional group/room/user id"),
            )
        }
    }

    impl SetDefault {
        pub fn app() -> App<'static> {
            clap::App::new("default-set")
                .about("Set the default menu")
                .arg(
                    Arg::new("id")
                        .short('i')
                        .required(false)
                        .takes_value(true)
                        .help("Optional group/room/user id"),
                )
                .arg(
                    Arg::new("menu")
                        .short('m')
                        .required(true)
                        .takes_value(true)
                        .help("Menu id"),
                )
        }
    }
}
