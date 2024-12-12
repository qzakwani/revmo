use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
};

use crate::{store, HOME_DIR};

const SEP_REGEX: &str = r"\[Desktop Action";
const NAME_REGEX: &str = r"Name=(.*?)(\n|$)";
const EXEC_REGEX: &str = r"Exec=(.*?)(\n|$)";
const ARGS_REGEX: &str = r"%[a-zA-Z]";
const ICON_REGEX: &str = r"Icon=(.*?)(\n|$)";

static APPS_LOCATIONS: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        "/usr/share/applications".to_string(),
        "/usr/local/share/applications".to_string(),
        format!("{}/.local/share/applications", *HOME_DIR),
        "/var/lib/flatpak/exports/share/applications/".to_string(),
        format!(
            "{}/.local/share/flatpak/exports/share/applications/",
            *HOME_DIR
        ),
        "/var/lib/snapd/desktop/applications/".to_string(),
    ]
});

static ICON_LOCATIONS: Lazy<HashMap<usize, Vec<String>>> = Lazy::new(|| {
    HashMap::from({
        [
            (
                0,
                vec![
                    "/usr/share/icons".to_string(),
                    "/usr/share/pixmaps".to_string(),
                ],
            ),
            (
                1,
                vec![
                    "/usr/local/share/icons".to_string(),
                    "/usr/local/share/pixmaps".to_string(),
                ],
            ),
            (
                2,
                vec![
                    format!("{}/.local/share/icons", *HOME_DIR),
                    format!("{}/.local/share/pixmaps", *HOME_DIR),
                ],
            ),
            (3, vec!["/var/lib/flatpak".to_string()]),
            (4, vec![format!("{}/.local/share/flatpak", *HOME_DIR)]),
            (5, vec!["/var/lib/snapd".to_string()]),
        ]
    })
});

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct App {
    pub id: usize,
    pub name: String,
    pub exec: String,
    pub args: Option<String>,
    pub action: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug)]
enum Error {
    EmptyFile,
    NameNotFound,
    ExecNotFound,
    RegexError,
    Other,
}

enum ProcessVar {
    Name,
    Exec,
    Icon,
}

macro_rules! unwrap_or_return_result {
    ($e:expr, $err:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => return Err($err),
        }
    };
}
macro_rules! unwrap_or_return_option {
    ($e:expr, $err:expr) => {
        match $e {
            Some(x) => x,
            None => return Err($err),
        }
    };
}

impl App {
    pub async fn get() -> Vec<App> {
        let _apps = store::get_apps();
        if let Some(apps) = _apps {
            return apps;
        }
        let mut apps = Vec::new();
        for (j, path) in APPS_LOCATIONS.iter().enumerate() {
            match read_dir(path) {
                Ok(mut files) => {
                    let mut ind = 0;
                    while let Some(file) = files.next() {
                        match file {
                            Ok(f) => {
                                ind += 1;
                                if ind > 10000 {
                                    break;
                                }
                                let path = f.path();
                                let p = path.to_str().unwrap();
                                if p.ends_with(".desktop") {
                                    match parse_desktop_file(p, j) {
                                        Ok(app) => {
                                            apps.extend(app);
                                        }
                                        Err(e) => {
                                            eprintln!("Error: {:?}", e);
                                            continue;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error: {:?}", e);
                                continue;
                            }
                        }
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        }

        apps.sort_by(|a, b| a.name.cmp(&b.name));
        for (i, app) in apps.iter_mut().enumerate() {
            app.id = i;
        }
        store::store_apps(&apps);
        apps
    }
}

// fn get_apps() -> Option<Vec<App>> {
//     let mut apps = Vec::new();
//     if let Ok(f) = std::fs::File::open("temp/apps.csv") {
//         let mut rdr = csv::Reader::from_reader(f);
//         for result in rdr.deserialize() {
//             if let Ok(app) = result {
//                 apps.push(app);
//             }
//         }
//     }
//     if apps.is_empty() {
//         None
//     } else {
//         Some(apps)
//     }
// }

// fn upsert_apps(apps: &[App]) -> Result<(), Error> {
//     if let Ok(f) = std::fs::File::create("temp/apps.csv") {
//         let mut wtr = csv::Writer::from_writer(f);
//         for app in apps {
//             wtr.serialize(app).unwrap_or(());
//         }
//         wtr.flush().unwrap_or(());
//         return Ok(());
//     }

//     Err(Error::IOError)
// }

fn parse_desktop_file(file: &str, j: usize) -> Result<Vec<App>, Error> {
    let contents = unwrap_or_return_result!(read_to_string(file), Error::Other);

    let re = unwrap_or_return_result!(Regex::new(SEP_REGEX), Error::RegexError);

    let mut apps = Vec::new();
    for (i, sec) in re.split(&contents).enumerate() {
        if sec.is_empty() && i == 0 {
            return Err(Error::EmptyFile);
        } else if sec.is_empty() {
            continue;
        }

        if i == 0 {
            apps.push(parse_section(sec, false, "", None, j)?);
        } else {
            apps.push(parse_section(
                sec,
                true,
                &apps[0].name,
                apps[0].icon.clone(),
                j,
            )?);
        }
    }

    Ok(apps)
}

fn parse_section(
    section: &str,
    is_action: bool,
    _name: &str,
    _icon: Option<String>,
    j: usize,
) -> Result<App, Error> {
    let re = unwrap_or_return_result!(Regex::new(NAME_REGEX), Error::RegexError);
    let _match = unwrap_or_return_option!(re.find(section), Error::NameNotFound);
    let name = process_match(_match, ProcessVar::Name);

    let re = unwrap_or_return_result!(Regex::new(EXEC_REGEX), Error::RegexError);
    let _match = unwrap_or_return_option!(re.find(section), Error::ExecNotFound);
    let mut exec = process_match(_match, ProcessVar::Exec);

    let mut icon = _icon;
    if !is_action {
        let re = unwrap_or_return_result!(Regex::new(ICON_REGEX), Error::RegexError);
        icon = match re.find(section) {
            Some(_match) => find_icon_path(&mut process_match(_match, ProcessVar::Icon), j),
            None => None,
        };
    }

    let re = unwrap_or_return_result!(Regex::new(ARGS_REGEX), Error::RegexError);
    let _args = re
        .find_iter(&exec)
        .map(|mat| mat.as_str().to_string())
        .collect::<Vec<String>>()
        .join(",");
    let args: Option<String> = if _args.is_empty() {
        None
    } else {
        exec = re.replace_all(&exec, "").to_string().trim().to_string();
        Some(_args)
    };

    let app_name = if is_action {
        _name.to_string()
    } else {
        name.clone()
    };
    Ok(App {
        id: 0,
        name: app_name.trim().to_string(),
        exec,
        icon,
        args,
        action: if is_action { Some(name) } else { None },
    })
}

fn process_match(_match: regex::Match<'_>, p: ProcessVar) -> String {
    match p {
        ProcessVar::Name => _match
            .as_str()
            .trim()
            .strip_prefix("Name=")
            .unwrap_or("")
            .to_string(),
        ProcessVar::Icon => _match
            .as_str()
            .trim()
            .to_string()
            .strip_prefix("Icon=")
            .unwrap_or("")
            .to_string(),
        ProcessVar::Exec => _match
            .as_str()
            .trim()
            .to_string()
            .strip_prefix("Exec=")
            .unwrap_or("")
            .to_string(),
    }
}

fn find_icon_path(icon: &mut String, j: usize) -> Option<String> {
    if icon.contains(".png") || icon.contains(".svg") {
        return Some(icon.trim().to_string());
    }
    icon.push('*');

    let mut icon_path = "".to_string();
    let mut i = 0;
    let mut found = false;
    for path in ICON_LOCATIONS.get(&j).unwrap_or(&vec![]) {
        i += 1;
        match std::process::Command::new("find")
            .arg(path)
            .arg("-name")
            .arg(&icon)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    if let Ok(_paths) = String::from_utf8(output.stdout) {
                        for _path in _paths.split('\n') {
                            if icon_path.is_empty() {
                                icon_path = if _path.ends_with(".png") || _path.ends_with(".svg") {
                                    _path.to_string()
                                } else {
                                    "".to_string()
                                };
                            } else if !found {
                                (icon_path, found) = check_optimal_path(&mut icon_path, _path);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if i == 3 {
                    eprintln!("Error: {:?}", e);
                    return None;
                } else {
                    continue;
                }
            }
        }
    }
    if icon_path.is_empty() {
        return None;
    }
    Some(icon_path.trim().to_string())
}

fn check_optimal_path<'a>(icon_path: &'a str, _path: &'a str) -> (String, bool) {
    if _path.contains("/64") {
        return (_path.to_string(), true);
    }
    if _path.contains("/48") {
        return (_path.to_string(), true);
    }
    // if _path.contains("/32") {
    //     return (_path.to_string(), true);
    // }
    // if _path.contains("/24") {
    //     return (_path.to_string(), true);
    // }
    (icon_path.to_string(), false)
}
