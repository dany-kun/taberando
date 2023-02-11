use clap::{App, AppSettings, Arg, ArgMatches};
use serde::Deserialize;
use serde::Serialize;

use server::line::api::LineApi;
use server::line::http::LineClient;
use server::line::json::RichMenu;

const COMMANDS: [Command; 6] = [
    Command {
        action: CommandAction::List,
        help: "List channel menus",
    },
    Command {
        action: CommandAction::Delete,
        help: "Delete channel menu",
    },
    Command {
        action: CommandAction::Create,
        help: "Create a channel menu",
    },
    Command {
        action: CommandAction::Default,
        help: "Get channel default menu",
    },
    Command {
        action: CommandAction::SetDefault,
        help: "Set channel default menu",
    },
    Command {
        action: CommandAction::SetAlias,
        help: "Set channel menu alias",
    },
];

#[tokio::main]
async fn main() {
    env_logger::init();
    let line_token_arg = Arg::new("line-token").short('t').takes_value(true);
    let mut menu_app = clap::App::new("menu")
        .about("Manage line menus")
        .setting(AppSettings::SubcommandRequiredElseHelp);
    for command in COMMANDS {
        let app: App<'static> = command.into();
        menu_app = menu_app.subcommand(app.arg(line_token_arg.clone()));
    }

    let matches = clap::App::new("line")
        .version("1.0.0")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(menu_app)
        .get_matches();

    if let Some(("menu", cmd_matches)) = matches.subcommand() {
        if let Some((action, m)) = cmd_matches.subcommand() {
            let command: CommandAction =
                serde_json::from_str(format!("{}{}{}", '"', action, '"').as_str()).unwrap();
            match command {
                CommandAction::List => {
                    let result = get_http_client(m).get_rich_menus().await.unwrap();
                    println!("{result:?}")
                }
                CommandAction::Delete => {
                    let menu_id = m.value_of("id").unwrap();
                    get_http_client(m).delete_rich_menu(menu_id).await.unwrap();
                    println!("Menu {menu_id} deleted")
                }

                CommandAction::Create => {
                    let json_path = m.value_of("json").unwrap();
                    let image_path = m.value_of("image").unwrap();
                    let content = std::fs::read_to_string(json_path).unwrap();
                    let menu = serde_json::from_str::<RichMenu>(&content).unwrap();
                    let image = std::fs::read(image_path).unwrap();
                    let client = get_http_client(m);
                    let result = client.create_rich_menu(&menu, image).await.unwrap();
                    println!("Created menu {result:?}");
                }

                CommandAction::Default => {
                    let result = get_http_client(m)
                        .get_default_menu(m.value_of("id"))
                        .await
                        .unwrap();
                    println!("{result}")
                }

                CommandAction::SetDefault => {
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
                            .map(|user_id| format!("user {user_id}"))
                            .unwrap_or_else(|| "default".to_string())
                    )
                }

                CommandAction::SetAlias => {
                    let menu_id = m.value_of("menu").unwrap();
                    let alias = m.value_of("alias").unwrap();
                    get_http_client(m)
                        .set_rich_menu_alias(menu_id, alias)
                        .await
                        .unwrap();
                    println!("Menu alias {alias} set for menu {menu_id}")
                }
            }
        }
    }
}

fn get_http_client(m: &ArgMatches) -> LineClient {
    let token = m
        .value_of_t("line-token")
        .or_else(|_| std::env::var("LINE_TOKEN"))
        .expect("Please specify a line token");
    LineClient::new(&token)
}

#[derive(Serialize, Deserialize)]
enum CommandAction {
    #[serde(rename(serialize = "list", deserialize = "list"))]
    List,
    #[serde(rename(serialize = "delete", deserialize = "delete"))]
    Delete,
    #[serde(rename(serialize = "create", deserialize = "create"))]
    Create,
    #[serde(rename(serialize = "get-default", deserialize = "get-default"))]
    Default,
    #[serde(rename(serialize = "set-default", deserialize = "set-default"))]
    SetDefault,
    #[serde(rename(serialize = "set-alias", deserialize = "set-alias"))]
    SetAlias,
}

struct Command<'a> {
    action: CommandAction,
    help: &'a str,
}

impl From<Command<'static>> for App<'static> {
    fn from(command: Command<'static>) -> Self {
        let action_name = serde_json::to_string(&command.action)
            .unwrap()
            .trim_matches('"')
            .to_string();
        let app = clap::App::new(action_name).about(command.help);
        match command.action {
            CommandAction::List => app,
            CommandAction::Delete => {
                app.arg(Arg::new("id").short('i').required(true).takes_value(true))
            }
            CommandAction::Create => app
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
                ),
            CommandAction::Default => app.arg(
                Arg::new("id")
                    .short('i')
                    .required(false)
                    .takes_value(true)
                    .help("Optional group/room/user id"),
            ),
            CommandAction::SetDefault => app
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
                ),

            CommandAction::SetAlias => app
                .arg(
                    Arg::new("alias")
                        .short('a')
                        .required(true)
                        .takes_value(true)
                        .help("Alias to set on the menu"),
                )
                .arg(
                    Arg::new("menu")
                        .short('m')
                        .required(true)
                        .takes_value(true)
                        .help("Menu id"),
                ),
        }
    }
}
