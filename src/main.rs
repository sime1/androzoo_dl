use std::{fs::File, error::Error, collections::HashMap, io::Write};
use serde::{Deserialize, Serialize};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Androzoo API key
    #[arg(short, long)]
    api_key: String,

    // yaml file containing the list of packages to download
    #[arg(short, long)]
    packages: String,

    // csv file with the Androzoo lists
    #[arg(short, long)]
    csv: String,

    // path to folder in which the files will be saved
    #[arg(short, long)]
    output: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Record {
    pkg_name: String,
    vercode: Option<i32>,
    sha256: String,
}

#[tokio::main]
async fn main() -> Result<(),  Box<dyn Error>> {
    let args = Args::parse();
    let pkg_file = File::open(args.packages)?;
    let pkgs: Vec<String> = serde_yaml::from_reader(pkg_file)?;
    let csv_file = File::open(args.csv)?;
    let mut raw_record = csv::StringRecord::new();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_file);
    let headers = rdr.headers()?.clone();
    let mut apps: HashMap<String, Record> = HashMap::new();
    let bar = ProgressBar::new(0);
    bar.set_style(ProgressStyle::with_template("[{elapsed_precise} - {pos}/{len}] {prefix} {bar:40} {msg}")?);
    bar.set_message("loading csv");
    while rdr.read_record(&mut raw_record)? {
        match raw_record.deserialize::<Record>(Some(&headers)) {
            Ok(mut r) => {
                for pattern in &pkgs {
                    if glob_match::glob_match(pattern, &r.pkg_name) {
                        r.vercode = match r.vercode {
                            Some(ver) => Some(ver),
                            None => Some(0)
                        };
                        if !apps.contains_key(&r.pkg_name) || apps[&r.pkg_name].vercode.unwrap() < r.vercode.unwrap() {
                            apps.insert(r.pkg_name.clone(), r.clone());
                        }
                    }
                }
            },
            Err(err) => println!("{}", err.to_string()),
        }
    }

    let apps: Vec<_> = apps.values().collect();
    bar.set_length(apps.len() as u64);
    std::fs::create_dir_all(&args.output)?;
    for app in apps {
        bar.set_message(app.pkg_name.clone());
        let filename = format!("{}/{}.apk", &args.output, &app.pkg_name);
        let mut out = File::create(filename)?;
        let url = format!("https://androzoo.uni.lu/api/download?apikey={}&sha256={}", &args.api_key, &app.sha256);
        let res = reqwest::get(url).await?.bytes().await?;
        out.write_all(&res[..])?;
        bar.inc(1)
    }
    Ok(())
}
