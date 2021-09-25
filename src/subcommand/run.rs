use crate::util;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::str::FromStr;

// dyer run --info/--debug/warn
#[derive(std::fmt::Debug)]
pub struct SubComRun {
    pub options: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct MetaData {
    modules: HashMap<String, Module>,
    pkgs: Vec<String>,
    ctype: String,
    base_dir: String,
    pub package_name: String,
}

impl MetaData {
    pub fn new() -> Self {
        MetaData {
            modules: HashMap::new(),
            pkgs: vec!["std".to_string()],
            ctype: String::new(),
            base_dir: "./".into(),
            package_name: String::new(),
        }
    }

    pub(crate) fn hash(&self) -> (bool, u64) {
        let paths = [
            "Cargo.toml",
            "middleware",
            "pipeline",
            "parser",
            "entity",
            "spider",
        ];
        let mut h = DefaultHasher::new();
        for path in paths.iter() {
            let path_ = if path == &"Cargo.toml" {
                format!("{}Cargo.toml", &self.base_dir)
            } else {
                format!("{}src/{}.rs", &self.base_dir, path)
            };
            let mut file = std::fs::File::open(&path_).unwrap();
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();
            buf.hash(&mut h);
        }
        let hash = h.finish();
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(format!("{}.dyertrace", &self.base_dir))
            .unwrap();
        let mut bf = String::new();
        f.read_to_string(&mut bf).unwrap();
        let old = bf.trim().parse::<u64>().unwrap_or(0);
        //println!("old: {}, new: {}", old, hash);
        if old != hash {
            let mut ff = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(format!("{}.dyertrace", &self.base_dir))
                .unwrap();
            let s = format!("{}", hash);
            ff.write(&s.as_bytes()).unwrap();
        }

        (old == hash, hash)
    }

    pub(crate) fn init(&mut self) {
        self.get_pkg();
        let paths = ["middleware", "pipeline", "parser", "entity", "spider"];
        let raw_pat = r"(?sm)^\s*#\[(?P<module>(middleware)|(pipeline)|(entity)|(spider)|(parser))(\(\s*(?P<key>\w+)\s*\))?\].*?(?P<typ>(fn)|(struct)|(enum))\s*(?P<ident>\w+)((?u-sm).*?\->.*?Option<(?P<ctyp>.*?)>)?";
        let ctype_pat = r"(?sm)^\s*#\[\s*pipeline\s*\(\s*open_pipeline\s*\)\s*\].*?fn\s*(?P<ident>\w+).*?Option<(?P<ctyp>.*?)>";

        for i in 0..paths.len() {
            let pat = regex::Regex::from_str(&raw_pat).unwrap();
            let path = format!("{}src/{}.rs", self.base_dir, paths[i]);
            let mut file = std::fs::File::open(&path).unwrap();
            let mut handles = HashMap::new();
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();
            for cap in pat.captures_iter(&buf) {
                //println!("cap {:?}", cap);
                let module = cap.name("module").unwrap().as_str();
                let value = cap.name("ident").unwrap().as_str().to_string();
                let key = if ["spider", "parser"].contains(&module) {
                    value.clone()
                } else {
                    cap.name("key").unwrap().as_str().to_string()
                };
                if paths[i] == "pipeline" && &key == "open_pipeline" {
                    let ctype = match cap.name("ctyp") {
                        Some(c) => c.as_str().to_string(),
                        None => {
                            let ctype_pat = regex::Regex::from_str(&ctype_pat).unwrap();
                            if let Some(c) = ctype_pat.captures(&buf) {
                                c.name("ctyp").unwrap().as_str().to_string()
                            } else {
                                panic!("failed to extract return type of `open_pipeline`");
                            }
                        }
                    };
                    //println!(" {:?}", ctype);
                    self.ctype = ctype;
                }
                handles.insert(key, value);
            }
            let module = Module { path, handles };
            self.modules.insert(paths[i].to_string(), module);
        }
    }

    pub fn get_pkg(&mut self) {
        let files = std::fs::read_dir(&self.base_dir)
            .unwrap()
            .map(|p| p.unwrap().path().to_str().unwrap().into())
            .collect::<Vec<String>>();
        if !files
            .iter()
            .fold(false, |acc, file| acc || file.contains(&"Cargo.toml"))
        {
            panic!("current directory must contain `Cargo.toml` file");
        }
        let path = format!("{}/Cargo.toml", self.base_dir);
        let mut pkgs = Vec::new();
        let file = std::fs::File::open(path).unwrap();
        let reader = BufReader::new(file);
        let pat = regex::Regex::new(r"^\s*([\w|-]+)\s*=\s*").unwrap();
        let pat1 = regex::Regex::new(r"^\s*name\s*=.*?(?P<pkg_name>[\w|-]+)").unwrap();
        let pat2 = regex::Regex::new(r"^\s*\[dependencies\]").unwrap();
        let pat3 = regex::Regex::new(r"^\s*\[.*?\]").unwrap();
        let mut in_content = false;
        for line in reader.lines() {
            let text = line.unwrap();
            if pat2.is_match(&text) {
                in_content = true;
            } else if !pat2.is_match(&text) && pat3.is_match(&text) {
                in_content = false;
            }
            if in_content {
                if let Some(t) = pat.captures(&text) {
                    let pkg = t.get(1).unwrap().as_str().trim().replace("-", "_");
                    pkgs.push(pkg)
                }
            }
            if pat1.is_match(&text) {
                let name = pat1
                    .captures(&text)
                    .unwrap()
                    .name("pkg_name")
                    .unwrap()
                    .as_str()
                    .replace("-", "_");
                self.package_name = name.into();
            }
        }
        self.pkgs.extend(pkgs);
        //println!("packages: {:?}", self.pkgs);
    }

    fn complete_path(&self) -> String {
        let pieces = self
            .ctype
            .split("::")
            .map(|piece| piece.trim())
            .collect::<Vec<&str>>();
        let subpath = pieces[0].to_string();
        if !self.pkgs.contains(&subpath) {
            panic!("The return type of `open_pipeline` must starts with one of `{}`, not subpath: `{}`", &self.pkgs.join(" "), subpath);
        }
        "".into()
    }

    pub fn get_pkg_list(&self) -> String {
        let list = self
            .pkgs
            .iter()
            .filter(|&ele| ele != "std")
            .map(|md| format!("extern crate {};", md))
            .collect::<Vec<String>>();
        list.join("\n")
    }

    pub fn make_main(&self) {
        let entity = self.modules.get("entity").expect("entity cannot be none");
        let entities = entity.handles.get("entities").unwrap();
        let targ_ = "Targ".to_string();
        let parg_ = "Parg".to_string();
        let targ = entity.handles.get("targ").unwrap_or(&targ_);
        let parg = entity.handles.get("parg").unwrap_or(&parg_);
        let spider = self
            .modules
            .get("spider")
            .unwrap()
            .handles
            .values()
            .collect::<Vec<&String>>()[0];
        let get_middleware_list = self.modules.get("middleware").unwrap().get_list();
        let get_pipeline_list = self.modules.get("pipeline").unwrap().get_list();
        let get_pipeline_map = self.modules.get("pipeline").unwrap().get_map();
        let get_middleware_map = self.modules.get("middleware").unwrap().get_map();
        let ctype = &self.ctype;
        let ctype_import = self.complete_path();
        let get_pkg_list = self.get_pkg_list();
        let package_name = &self.package_name;

        let main_str = r"//#![allow(unused_imports)]

<+get_pkg_list+>
extern crate <+package_name+>; 

use dyer::*;
use <+package_name+>::entity::{<+entities+>, <+targ+>, <+parg+>};
use <+package_name+>::<+spider+>;
use <+package_name+>::middleware::{<+get_middleware_list+>};
use <+package_name+>::pipeline::{<+get_pipeline_list+>};
use std::sync::{Arc, Mutex};
<+ctype_import+>

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();
    let middleware = plug!( MiddleWare<<+entities+>, <+targ+>, <+parg+>> {
        <+get_middleware_map+>
    });
    let pipeline = plug!( PipeLine<<+entities+>, <+ctype+>> {
        <+get_pipeline_map+>
    } );
    let spider = <+spider+>::new();
    let mut app = dyer::App::<<+entities+>, <+targ+>, <+parg+>>::new();
    app.run(&spider, &middleware, pipeline).await.unwrap();
}
        ";
        let main_str = main_str.replace("<+package_name+>", &package_name);
        let main_str = main_str.replace("<+entities+>", &entities);
        let main_str = main_str.replace("<+targ+>", &targ);
        let main_str = main_str.replace("<+parg+>", &parg);
        let main_str = main_str.replace("<+spider+>", &spider);
        let main_str = main_str.replace("<+get_pkg_list+>", &get_pkg_list);
        let main_str = main_str.replace("<+get_middleware_list+>", &get_middleware_list);
        let main_str = main_str.replace("<+get_middleware_map+>", &get_middleware_map);
        let main_str = main_str.replace("<+get_pipeline_list+>", &get_pipeline_list);
        let main_str = main_str.replace("<+get_pipeline_map+>", &get_pipeline_map);
        let main_str = main_str.replace("<+ctype+>", ctype);
        let main_str = main_str.replace("<+ctype_import+>", &ctype_import);
        let main_path = format!("{}src/bin/{}.rs", self.base_dir, package_name);
        let mut main_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(main_path)
            .unwrap();
        main_file.write(&main_str.as_bytes()).unwrap();
    }
}

#[derive(Debug)]
struct Module {
    path: String,
    handles: HashMap<String, String>,
}

impl Module {
    pub fn get_list(&self) -> String {
        self.handles
            .values()
            .map(|val| val.as_str())
            .collect::<Vec<&str>>()
            .join(", ")
    }

    pub fn get_map(&self) -> String {
        self.handles
            .iter()
            .map(|(key, val)| format!("{}: {}", key, val))
            .collect::<Vec<String>>()
            .join(",\n        ")
    }
}

impl SubComRun {
    pub fn execute(&self) {
        let paths = std::fs::read_dir("./src/bin")
            .unwrap()
            .map(|p| p.unwrap().path().to_str().unwrap().into())
            .collect::<Vec<String>>();
        //println!("files in \"./\" {:?}", paths);
        let pkg_name = util::get_package_name();
        if !paths
            .iter()
            .fold(false, |acc, x| acc || x.contains(&pkg_name))
        {
            let mut meta = MetaData::new();
            meta.init();
            //println!("{:?}", meta);
            meta.make_main();
        }
        let options = self
            .options
            .iter()
            .map(|op| op.as_str())
            .filter(|op| {
                if ["--off", "--error", "--warn", "--info", "--debug", "--trace"].contains(&op) {
                    util::change_log_level(op);
                    return false;
                }
                true
            })
            .collect::<Vec<&str>>();
        let mut args = vec!["run"];
        args.extend(options);
        util::run_command("cargo", args);
    }
}
