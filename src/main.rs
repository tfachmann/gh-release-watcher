use regex::Regex;
use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize, Serializer};
use standard_paths::{LocationType, StandardPaths};
use std::io::prelude::*;
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    process::Command,
    thread, time,
};
use std::{path::PathBuf, process::Stdio};
use toml;
use url::Url;

fn parse_repo(url: &str, key: &str, version: &str, email: &str) -> Option<String> {
    let mut resp = reqwest::get(url).unwrap();
    assert!(resp.status().is_success());

    let body = resp.text().unwrap();
    let fragment = Html::parse_document(&body);
    let title = Selector::parse("title").unwrap();
    let re = Regex::new(r"\d\.\d\.\d").unwrap();

    let mut title_it = fragment.select(&title);
    let version_new = match title_it.next() {
        Some(x) => match x.text().next() {
            Some(v) => Some(re.find(v).unwrap().as_str()),
            None => None,
        },
        None => None,
    }
    .unwrap();
    if version_new == version {
        println!("Nothing happend...");
        return None;
    }
    // new version!
    println!("New Version! {}", version_new);

    // check date of release
    let time = Selector::parse("relative-time").unwrap();
    let mut time_it = fragment.select(&time);
    let datetime = match time_it.next() {
        Some(x) => match x.text().next() {
            Some(v) => Some(v),
            None => None,
        },
        None => None,
    };

    // send email
    let email_text = format!("The repository `{}` has released version `{}` on {}!\nGo check it out {}.\n\ngh-release-watcher", key, version_new, datetime.unwrap(), url);
    let cmd = Command::new("mail")
        .arg("-s")
        .arg("'New Release!'")
        .arg(email)
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to spawn mail!");
    cmd.stdin
        .unwrap()
        .write_all(email_text.as_bytes())
        .expect("Failed to send mail!");

    // recognize that message is not sent

    // update config / HashMap
    Some(version_new.to_string())
}

struct Application {
    config: Config<Github, Gitlab>,
    path: PathBuf,
}

impl Application {
    fn new(path: &PathBuf) -> Result<Self, std::io::Error> {
        let data_str = fs::read_to_string(path)
            .expect(&format!("Could not open `{}`", path.to_str().unwrap()));
        let config: Config<Github, Gitlab> = toml::from_str(&data_str)?;
        Ok(Self {
            config,
            path: path.clone(),
        })
    }

    fn save(&self) {
        let data_str = toml::to_string(&self.config).unwrap();
        fs::write(&self.path, data_str).unwrap();
    }

    fn run(&mut self) {
        self.config.print_info();

        // start loop
        loop {
            if self.config.check_repos() {
                self.save();
            }
            thread::sleep(time::Duration::from_secs(self.config.config.time));
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Configurables {
    email: String,
    #[serde(default = "default_time")]
    time: u64,
}

fn default_time() -> u64 {
    3600
}

#[derive(Serialize, Deserialize)]
struct Config<T, U>
where
    T: UrlGroup,
    U: UrlGroup,
{
    github: Option<T>,
    gitlab: Option<U>,
    config: Configurables,
}

impl<T: UrlGroup + Clone, U: UrlGroup + Clone> Config<T, U> {
    fn print_info(&self) {
        fn print_group<V: UrlGroup>(x: &Option<V>) {
            if let Some(v) = x {
                v.data()
                    .iter()
                    .for_each(|(key, _)| println!("Watching `{}`...", v.url(&key)));
            }
        }

        print_group(&self.github);
        print_group(&self.gitlab);
    }

    fn check_repos(&mut self) -> bool {
        #[must_use]
        fn check_group<V: UrlGroup + Clone>(x: &mut Option<V>, email: &str) -> bool {
            let mut changed = false;
            if let Some(v) = x {
                let x_cl = v.clone();
                for (key, val) in v.data_mut().iter_mut() {
                    if let Some(x) = parse_repo(&x_cl.url(&key), &key, &val, email) {
                        *val = x;
                        changed = true;
                    }
                }
            }
            changed
        }

        if check_group(&mut self.github, &self.config.email)
            || check_group(&mut self.gitlab, &self.config.email)
        {
            true
        } else {
            false
        }
    }
}

trait UrlGroup {
    fn data(&self) -> &HashMap<String, String>;
    fn data_mut(&mut self) -> &mut HashMap<String, String>;
    fn base(&self) -> &str;
    fn url(&self, repo: &str) -> String {
        let mut base = self.base().to_string();
        if base.chars().last().unwrap() != '/' {
            base.push('/');
        }
        let mut name = repo.to_string();
        if repo.chars().last().unwrap() != '/' {
            name.push('/');
        }
        let url = Url::parse(&base).unwrap();
        url.join(&name)
            .unwrap()
            .join("releases/latest")
            .unwrap()
            .to_string()
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Github {
    #[serde(flatten, serialize_with = "ordered_map")]
    entries: HashMap<String, String>,
}

impl UrlGroup for Github {
    fn base(&self) -> &str {
        "https://github.com"
    }
    fn data(&self) -> &HashMap<String, String> {
        &self.entries
    }
    fn data_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.entries
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Gitlab {
    #[serde(flatten, serialize_with = "ordered_map")]
    entries: HashMap<String, String>,
}

impl UrlGroup for Gitlab {
    fn base(&self) -> &str {
        "https://gitlab.com"
    }
    fn data(&self) -> &HashMap<String, String> {
        &self.entries
    }
    fn data_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.entries
    }
}

fn ordered_map<S>(value: &HashMap<String, String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

fn main() {
    // load config
    let sp = StandardPaths::new_with_names("gh-release-watcher", "");
    let data_base = sp
        .writable_location(LocationType::AppConfigLocation)
        .unwrap()
        .join("config.toml");
    let mut config = Application::new(&data_base).unwrap();
    // register watcher
    config.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_test() {
        let test_str = r###"
[github]
"neovim/neovim" = "4.4.0"
"test/bla" = "0.1.2"

[config]
email = "bla@email.com"
time = 13
"###;
        let config: Config<Github, Gitlab> = toml::from_str(test_str).unwrap();
        assert_eq!(config.config.email, "bla@email.com".to_string());
        assert_eq!(config.config.time, 13);
        assert!(config.gitlab.is_none());
        assert_eq!(config.github.unwrap().entries.len(), 2);
    }

    #[test]
    fn serialize_test() {
        let entries: HashMap<String, String> = [
            ("neovim/neovim".to_string(), "4.4.0".to_string()),
            ("test/bla".to_string(), "0.1.2".to_string()),
        ]
        .iter()
        .cloned()
        .collect();
        let github = Github { entries };

        let config: Config<Github, Gitlab> = Config {
            github: Some(github),
            gitlab: None,
            config: Configurables {
                email: "bla@email.com".to_string(),
                time: 13,
            },
        };
        assert_eq!(
            toml::to_string(&config).unwrap(),
            r###"[github]
"neovim/neovim" = "4.4.0"
"test/bla" = "0.1.2"

[config]
email = "bla@email.com"
time = 13
"###
        );
    }
}
