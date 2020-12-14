use clap::{crate_authors, crate_version, App, AppSettings, Arg};
use clap_generate::{
    generate,
    generators::{Bash, Elvish, Fish, PowerShell, Zsh},
};
use crossbeam_channel::Receiver;
use lazy_static::lazy_static;
use log::{debug, info};
use reqwest::blocking::ClientBuilder;
use std::{
    collections::{HashMap, HashSet},
    io, process,
    sync::Mutex,
    thread,
    time::Duration,
};

use crate::http::ClipboardResponse;

mod clipboard;
mod http;

const SERVICE_DEFAULT_PORT: u16 = 8000;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

lazy_static! {
    pub static ref LAST_CLIPBOARD_CONTENT: Mutex<String> = Mutex::new(String::new());
    pub static ref CURRENT_PORT: Mutex<u16> = Mutex::new(SERVICE_DEFAULT_PORT);
    pub static ref DISCOVERED_SERVICES: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

fn fetch_updates(host: &str, port: u16) {
    let poll_url = format!("http://{}:{}/get_clipboard", host, port);

    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    loop {
        let response = match client.get(&poll_url).send() {
            Ok(r) => r,
            Err(_) => continue,
        };

        match response.json::<ClipboardResponse>().unwrap().contents {
            Some(response) => clipboard::clipboard_update(response),
            None => continue,
        };
    }
}

fn client_mode(host: &str, port: u16, receiver: Receiver<String>) {
    info!("run in client mode");
    let fetch_host = host.to_string();
    thread::spawn(move || fetch_updates(&fetch_host, port));

    let poll_url = format!("http://{}:{}/push_clipboard", host, port);
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();

    debug!("enter in the loop");
    loop {
        let update = receiver.recv().unwrap();
        debug!("got update: {:?}", update);
        let mut map = HashMap::new();
        map.insert("contents", update);

        debug!("try update server {}:{}", host, port);
        client.post(&poll_url).json(&map).send().unwrap();
    }
}

fn server_mode(host: &str, port: u16, receiver: Receiver<String>) {
    info!("run in server mode");
    http::server(&host, port, receiver).unwrap()
}

fn autocomplete(shell: &str, mut app: &mut App) {
    match shell {
        "bash" => generate::<Bash, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        "elvish" => generate::<Elvish, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        "fish" => generate::<Fish, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        "powershell" => generate::<PowerShell, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        "zsh" => generate::<Zsh, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        _ => panic!("Unknown generator"),
    }
}

fn main() {
    flexi_logger::Logger::with_env_or_str("info")
        .start()
        .expect("logger");

    let (sender, receiver) = crossbeam_channel::unbounded();

    let mut app = App::new("clipboard-sync")
        .setting(AppSettings::SubcommandsNegateReqs)
        .author(crate_authors!())
        .version(crate_version!())
        .about("Sync clipboard over HTTP")
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .takes_value(true)
                .required_if_eq("mode", "client")
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .takes_value(true)
                .default_value("8000"),
        )
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .required(true)
                .takes_value(true)
                .possible_values(&["server", "client"]),
        )
        .subcommand(
            App::new("init")
                .about("Prints the shell function used to execute")
                .arg(
                    Arg::new("shell")
                        .value_name("SHELL")
                        .takes_value(true)
                        .required(true)
                        .possible_values(&["bash", "elvish", "fish", "powershell", "zsh"]),
                ),
        );

    let matches = app.clone().get_matches();

    if let Some(init) = matches.subcommand_matches("init") {
        autocomplete(init.value_of("shell").unwrap(), &mut app);
        process::exit(0);
    }

    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap().parse().unwrap();

    thread::spawn(move || clipboard::clipboard_loop(sender).unwrap());

    match matches.value_of("mode") {
        Some("server") => server_mode(host, port, receiver),
        Some("client") => client_mode(host, port, receiver),
        _ => unreachable!(),
    }
}
