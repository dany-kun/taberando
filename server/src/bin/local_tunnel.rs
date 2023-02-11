extern crate server;

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use regex::Regex;

use server::line::api::LineApi;
use server::line::http::LineClient;

// TODO Refactor this to use concurrency primitives

fn main() {
    env_logger::init();
    let port = std::env::var("WEBHOOK_LOCAL_PORT").map_or(4010, |v| v.parse::<i32>().unwrap());
    let line_token = std::env::var("LINE_TOKEN").unwrap();
    open_local_url(port, line_token);
}

fn open_local_url(port: i32, line_token: String) {
    let child = Command::new("ngrok")
        .arg("http")
        .arg(port.to_string())
        .arg("--log")
        .arg("stdout")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| panic!("failed to execute lt opening port process on port {port}"));

    let out = BufReader::new(child.stdout.unwrap());

    out.lines().for_each(|line| {
        let string = line.unwrap();
        parse_output(string, line_token.as_str());
    });
}

fn parse_output(output: String, line_token: &str) {
    let lt_url_regex: Regex = Regex::new(r"url=https(\S+)").unwrap();

    match lt_url_regex.captures(&output) {
        Some(matches) => {
            let result = matches.get(1).map_or("", |m| m.as_str()).trim();
            let scheme_result = format!("https{result}");
            println!("Exposing localhost to {scheme_result}");
            handle_public_url(line_token, &scheme_result);
        }
        None => println!("Got unhandled output from lt: \"{output}\""),
    };
}

fn handle_public_url(line_token: &str, result: &str) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            LineClient::new(line_token)
                .update_line_webhook_url(result)
                .await
                .unwrap();
            store_in_file(result)
        });
}

fn store_in_file(url: &str) {
    std::fs::write("./src/public.url", url).unwrap();
}
