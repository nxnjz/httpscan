use clap::{App, Arg};
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::header;
use reqwest::Proxy;
use reqwest::Url;
use scraper::Html;
use scraper::Selector;
use serde_json;

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::BufRead;
use std::io::Read;
use std::io::Stdin;
use std::time::Duration;

fn html_title(html: &str) -> Option<String> {
    let fragment = Html::parse_fragment(&html);
    let selector = Selector::parse("title").unwrap();
    let selection = fragment.select(&selector).nth(0);
    selection.map(|x| x.inner_html())
}

fn thread(client: Client, mut urls: Vec<String>) {
    for urlstr in urls.drain(..) {
        let url = Url::parse(&urlstr);
        if let Ok(url) = url {
            let r = client.get(url.clone()).send();
            let mut results = HashMap::new();
            results.insert("url", urlstr);
            let host = url.host_str();
            if let Some(host) = host {
                results.insert("fqdn", host.to_owned());
                if let Ok(resp) = r {
                    results.insert("success", "true".to_owned());
                    results.insert("status_code", resp.status().as_str().to_owned());
                    results.insert("final_url", resp.url().to_string());
                    if let Ok(text) = resp.text() {
                        results.insert("response_length", text.len().to_string());
                        results.insert("html_title", html_title(&text).unwrap_or("".to_owned()));
                    } else {
                        results.insert("response_length", "0".to_string());
                        results.insert("html_title", "".to_owned());
                    }
                } else {
                    results.insert("success", "false".to_owned());
                }
                println!("{}", serde_json::to_string(&results).unwrap());
            }
        }
    }
}

fn main() {
    let matches = App::new("httpscan")
        .version("0.0.1")
        .author("nxnjz")
        .about("Reads URLs from stdin and returns response information as JSON")
        .arg(
            Arg::with_name("timeout")
                .short("T")
                .long("timeout")
                .help("Total request timeout in milliseconds")
                .takes_value(true)
                .default_value("15000"),
        )
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .help("Number of threads")
                .takes_value(true)
                .default_value("20"),
        )
        .arg(
            Arg::with_name("proxy")
                .short("x")
                .long("proxy")
                .help("Proxy URL")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("proxy_auth")
                .short("U")
                .long("proxy-auth")
                .help("Proxy authentication as username:password")
                .takes_value(true),
        )
        .get_matches();

    let timeout: u64 = matches.value_of("timeout").unwrap().parse().unwrap();
    let threads: u64 = matches.value_of("threads").unwrap().parse().unwrap();
    //let retries: u64 = matches.value_of("retries").unwrap().parse().unwrap();
    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.149 Safari/537.36"));
    headers.insert(
        header::CONNECTION,
        header::HeaderValue::from_static("close"),
    );
    let mut client = ClientBuilder::new()
        .timeout(Duration::from_millis(timeout))
        .danger_accept_invalid_certs(true)
        .default_headers(headers);

    if let Some(pxy_url) = matches.value_of("proxy") {
        let mut proxy = Proxy::all(pxy_url).unwrap();
        if let Some(pxy_auth) = matches.value_of("proxy_auth") {
            let u = pxy_auth.split(':').nth(0).expect("invalid proxy auth");
            let p = pxy_auth.split(':').nth(1).expect("invalid proxy auth");
            proxy = proxy.basic_auth(u, p);
        }
        client = client.proxy(proxy);
    }
    let client = client.build().unwrap();

    let stdin = std::io::stdin();
    let mut urls = stdin
        .lock()
        .lines()
        .map(|x| x.unwrap())
        .collect::<Vec<String>>();
    let mut urls_split = Vec::new();
    for _ in 0..threads {
        urls_split.push(Vec::new());
    }
    while urls.len() > 0 {
        for i in 0..threads {
            let i = i as usize;
            if let Some(url) = urls.pop() {
                urls_split[i].push(url);
            }
        }
    }
    let mut handles = Vec::new();
    for i in 0..threads {
        let urls = urls_split.pop().unwrap();
        let client = client.clone();
        handles.push(std::thread::spawn(move || thread(client, urls)));
    }
    for handle in handles.drain(..) {
        handle.join();
    }
}
