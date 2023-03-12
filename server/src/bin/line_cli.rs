use clap::{Arg, ArgMatches};
use serde::Deserialize;
use serde::Serialize;

use server::line::api::LineApi;
use server::line::http::LineClient;
use server::line::json::RichMenu;

const COMMANDS: [CommandAction; 6] = [
    CommandAction::List,
    CommandAction::Delete,
    CommandAction::Create,
    CommandAction::Default,
    CommandAction::SetDefault,
    CommandAction::SetAlias,
];

#[tokio::main]
async fn main() {
    env_logger::init();
    let line_token_arg = Arg::new("line-token").short('t');
    let mut menu_app = clap::Command::new("menu")
        .about("Manage line menus")
        .subcommand_required(true)
        .arg_required_else_help(true);
    for command in COMMANDS {
        let app: clap::Command = command.into();
        menu_app = menu_app.subcommand(app.arg(line_token_arg.clone()));
    }

    let matches = clap::Command::new("line")
        .version("1.0.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
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
                    let menu_id = m.get_one::<String>("id").unwrap();
                    get_http_client(m).delete_rich_menu(menu_id).await.unwrap();
                    println!("Menu {menu_id} deleted")
                }

                CommandAction::Create => {
                    let json_path = m.get_one::<String>("json").unwrap();
                    let image_path = m.get_one::<String>("image").unwrap();
                    let content = std::fs::read_to_string(json_path).unwrap();
                    let menu = serde_json::from_str::<RichMenu>(&content).unwrap();
                    let image = std::fs::read(image_path).unwrap();
                    let client = get_http_client(m);
                    let result = client.create_rich_menu(&menu, image).await.unwrap();
                    println!("Created menu {result:?}");
                }

                CommandAction::Default => {
                    let result = get_http_client(m)
                        .get_default_menu(m.get_one::<String>("id").map(|a| a.as_str()))
                        .await
                        .unwrap();
                    println!("{result}")
                }

                CommandAction::SetDefault => {
                    let menu_id = m.get_one::<String>("menu").unwrap();
                    let user_id = m.get_one::<String>("id").map(|a| a.as_str());
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
                    let menu_id = m.get_one::<String>("menu").unwrap();
                    let alias = m.get_one::<String>("alias").unwrap();
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
        .get_one::<String>("line-token")
        .map(|a| a.to_string())
        .or_else(|| std::env::var("LINE_TOKEN").ok())
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

impl CommandAction {
    fn help(&self) -> &str {
        match self {
            CommandAction::List => "List channel menus",
            CommandAction::Delete => "Delete channel menu",
            CommandAction::Create => "Create a channel menu",
            CommandAction::Default => "Get channel default menu",
            CommandAction::SetDefault => "Set channel default menu",
            CommandAction::SetAlias => "Set channel menu alias",
        }
    }
}

impl From<CommandAction> for clap::Command {
    fn from(action: CommandAction) -> Self {
        let command_name = serde_json::to_string(&action)
            .map(|a| a.trim_matches('"').to_string())
            .unwrap();
        let app = clap::Command::new(command_name).about(action.help().to_string());
        match action {
            CommandAction::List => app,
            CommandAction::Delete => app.arg(Arg::new("id").short('i').required(true)),
            CommandAction::Create => app
                .arg(
                    Arg::new("json")
                        .help("Path to the source json file")
                        .short('j')
                        .required(true),
                )
                .arg(
                    Arg::new("image")
                        .help("Path to the source image file")
                        .short('i')
                        .required(true),
                ),
            CommandAction::Default => app.arg(
                Arg::new("id")
                    .short('i')
                    .required(false)
                    .help("Optional group/room/user id"),
            ),
            CommandAction::SetDefault => app
                .arg(
                    Arg::new("id")
                        .short('i')
                        .required(false)
                        .help("Optional group/room/user id"),
                )
                .arg(Arg::new("menu").short('m').required(true).help("Menu id")),

            CommandAction::SetAlias => app
                .arg(
                    Arg::new("alias")
                        .short('a')
                        .required(true)
                        .help("Alias to set on the menu"),
                )
                .arg(Arg::new("menu").short('m').required(true).help("Menu id")),
        }
    }
}
