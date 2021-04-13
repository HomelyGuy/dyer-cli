use std::str::FromStr;

#[derive(std::fmt::Debug)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
impl FromStr for LogLevel {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(()),
        }
    }
}

pub fn get_file_path<'a>(index: &'a str, name: String) -> String {
    match index {
        "readme" => name + "/README.md",
        "cargo" => name + "/Cargo.toml",
        "entity" => name + "/src/entity.rs",
        "parser" => name + "/src/parser.rs",
        "spider" => name + "/src/spider.rs",
        "middleware" => name + "/src/middleware.rs",
        "main" => name + "/src/main.rs",
        "pipeline" => name + "/src/pipeline.rs",
        _ => panic!(),
    }
}
pub fn get_file_intro(index: &str) -> &str {
    match index {
        "readme" => {
            r#"// This is a markdown file generated by dyer-cli
// Instructions of the project specified here"#
        }
        "entity" => {
            r#"// define data structure here to be used or collected
// all data structures got to be Serializable and Deserializable

use serde::{Deserialize, Serialize};

// the Entity to be used
/*
 *#[derive(Deserialize, Serialize, Debug, Clone)]
 *pub struct Item1 {
 *    pub field1: String,
 *    pub field2: i32,
 *}
 */

// serve as a placeholder for all entities, and generic parameter of dyer::App
#[derive(Serialize, Debug, Clone)]
pub enum Entities {
    //Item1(Item1),
}

// serve as a appendix/complement to dyer::Task
// providing more infomation for this Task, leave it empty if not necessary
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Targ {}

// serve as a appendix/complement to dyer::Profile
// providing more infomation for this Profile, empty as default
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Parg {}"#
        }
        "main" => {
            r#"extern crate dyer;
extern crate serde;
extern crate simple_logger;
extern crate tokio;

mod entity;
mod middleware;
mod parser;
mod pipeline;
mod spider;

use dyer::{log, App};
use entity::*;
use middleware::get_middleware;
use pipeline::get_pipeline;
use spider::MySpider;

#[tokio::main]
async fn main() {
    // initialize simple_logger use simple_logger to display some level-varied infomation
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        //.with_module_level("dyer", log::LevelFilter::Debug) // log level varied from modules
        .init()
        .unwrap();
    let spd: MySpider = MySpider {};
    // initialize the middleware
    let middleware = get_middleware();
    // initialize the pipeline
    let pipeline = get_pipeline();
    // construct the app and start the crawler
    let mut app: App<Entities, Targ, Parg> = App::<Entities, Targ, Parg>::new();
    // AppArg configuration, custiomize your app including:
    // rate control, history file usage, app load balance and so on
    // more details see https://docs.rs/dyer/engine/struct.AppArg.html
    app.rt_args.lock().unwrap().skip_history = true;
    /*
     *app.rt_args.lock().unwrap().round_req_max = 100;
     *app.rt_args.lock().unwrap().gap = 10;
     */
    app.run(&spd, &middleware, pipeline).await.unwrap();
}"#
        }
        "middleware" => {
            r#"// Middleware that processes the data before reaching PipeLine
// including dealing with errors, data structures, runtime modification

use crate::entity::{Entities, Parg, Targ};
use dyer::{plug, App, FutureExt, MiddleWare};

// there are 7 methods availible:
//     1. hand_profile
//     2. hand_task
//     3. hand_req
//     4. hand_res
//     5. hand_item
//     6. hand_err
//     7. hand_yerr
// you can specify some of them if necessary, others are assigned as default
// More details in https://docs.rs/dyer/plugin/middleware/struct.MiddleWare.html

// process Entities if necessary
pub async fn hand_item(_items: &mut Vec<Entities>, _app: &mut App<Entities, Targ, Parg>) {}

pub fn get_middleware<'md>() -> MiddleWare<'md, Entities, Targ, Parg> {
    plug!( MiddleWare<Entities, Targ, Parg> {
        hand_item: hand_item,
    })
}"#
        }
        "parser" => {
            r#"// Parsers that extract entities from Response
// external tool may be used to achieve that

use crate::entity::{Entities, Parg, Targ};
use dyer::{ParseResult, Response};

// note that call this function to parse via specifying:
//     let task = Task::default();
//     ...
//     task.parser = "parse_index".to_string();
// that means function `parse_index` is called once the task being executed successfully and
// becoming response.
pub fn parse_index(_res: Response<Targ, Parg>) -> ParseResult<Entities, Targ, Parg> {
    ParseResult::default()
}"#
        }
        "pipeline" => {
            r#"// PipeLine that consume all entities, the end of data flow
// stdout the data as default, customaization is need for data storage

// there 4 methods availible:
//     1. open_pipeline
//     2. close_pipeline
//     3. process_item
//     4. process_yerr
// more details see https://docs.rs/dyer/plugin/pipeline/struct.PipeLine.html

use crate::entity::Entities;
use dyer::{plug, FutureExt, PipeLine};

// something to do before sending entities to pipeline
// note that this function only runs one time
async fn open_pipeline<'a>() -> &'a Option<std::fs::File> {
    &None
}

pub fn get_pipeline<'pl>() -> PipeLine<'pl, Entities, std::fs::File> {
    plug!(PipeLine<Entities, std::fs::File> {
        open_pipeline: open_pipeline,
    })
}"#
        }
        "spider" => {
            r#"// Set up initial condition when stepping into Spider and work to do when closing spider

use crate::entity::{Entities, Parg, Targ};
use crate::parser::*;
use dyer::{plug, App, ParseResult, ProfileInfo, Request, Response, Spider, Task};

type Stem<U> = Result<U, Box<dyn std::error::Error + Send + Sync>>;
type Btem<E, T, P> = dyn Fn(Response<T, P>) -> ParseResult<E, T, P>;

pub struct MySpider {}

impl Spider<Entities, Targ, Parg> for MySpider {
    // preparation before opening spider
    fn open_spider(&self, _app: &mut App<Entities, Targ, Parg>) {}

    // `Task` to be executed when starting `dyer`. Note that this function must reproduce a
    // non-empty vector, if not, the whole program will be left at blank.
    fn entry_task(&self) -> Stem<Vec<Task<Targ>>> {
        Ok(vec![])
    }

    // the generator of `Profile`
    // `dyer` consume the returned `Request`, generate a `Response` fed to the closure
    // to generate a `Profile`
    fn entry_profile<'a>(&self) -> ProfileInfo<'a, Targ, Parg> {
        ProfileInfo {
            req: Request::<Targ, Parg>::default(),
            parser: None,
        }
    }

    // set up parser that extracts `Entities` from the `Response`
    // by the name of Task.parser return the parser function
    //parser is indexed by a `String` name, like:
    //task.parser = "parse_quote".to_string();
    fn get_parser<'a>(&self, ind: String) -> Option<&'a Btem<Entities, Targ, Parg>> {
        plug!(get_parser(ind; parse_index))
    }

    // preparation before closing spider
    fn close_spider(&self, _app: &mut App<Entities, Targ, Parg>) {}
}"#
        }
        "cargo" => {
            r#"[package]
name = "<+name+>"
version = "0.1.0"
authors = ["<+author+>"]
edition = "2018"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
dyer = { version = "*" }
serde = { version = "*", features = ["derive"] }
tokio = { version = "0.2", features = ["rt-threaded", "macros"]}
simple_logger = "*""#
        }
        _ => "",
    }
}