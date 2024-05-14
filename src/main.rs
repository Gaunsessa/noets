#![feature(lazy_cell)]

#[macro_use]
extern crate rouille;

use std::hash::{Hasher, BuildHasher};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::iter::zip;
use std::io::Read;
use std::fs;
use std::io;

const NOTE_HTML: &'static str = include_str!("../res/note.html");
const EDIT_HTML: &'static str = include_str!("../res/edit.html");
static E404_HTML: &'static str = include_str!("../res/404.html");

const ADJS_STR: &'static str = include_str!("../res/adjs");
const VRBS_STR: &'static str = include_str!("../res/vrbs");

static ADJS: LazyLock<(HashMap<&'static str, u8>, HashMap<u8, &'static str>)> = LazyLock::new(|| {
   (zip(ADJS_STR.split("\n"), 0..255).collect(), zip(0..255, ADJS_STR.split("\n")).collect())
});

static VRBS: LazyLock<(HashMap<&'static str, u8>, HashMap<u8, &'static str>)> = LazyLock::new(|| {
   (zip(VRBS_STR.split("\n"), 0..255).collect(), zip(0..255, VRBS_STR.split("\n")).collect())
});

macro_rules! tryorr {
   ($res:expr) => {
      match $res {
         Ok(x) => x,
         Err(_) => {
            return rouille::Response::html(E404_HTML).with_status_code(404);
         }
      }
   }
}

fn wurltonurl(url: String) -> Option<String> {
   let chunks = url.split("-").collect::<Vec<&str>>();

   if chunks.len() != 3 { return None }

   Some(format!("{:02x}{:02x}{:02x}", ADJS.0.get(chunks[0])?, ADJS.0.get(chunks[1])?, VRBS.0.get(chunks[2])?))
}

fn nurltowurl(url: String) -> Option<String> {
   if url.len() != 6 { return None }

   let chunks: Result<Vec<u8>, std::num::ParseIntError> = url.chars()
      .collect::<Vec<char>>()
      .chunks(2)
      .map(|x| u8::from_str_radix(&String::from_iter(x), 16))
      .collect();

   if let Ok(chunks) = chunks {
      Some(format!("{}-{}-{}", ADJS.1.get(&chunks[0])?, ADJS.1.get(&chunks[1])?, VRBS.1.get(&chunks[2])?))
   } else {
      None
   }
}

fn get_note_html(name: String) -> Option<String> {
   if let Ok(file) = fs::read_to_string(format!("notes/{}", name)) {
      let res = NOTE_HTML
         .replace("##NOTETEXT##", &file)
         .replace("##NOTEBURL##", &format!("{}", &name))
         .replace("##NOTERURL##", &format!("raw/{}", &name))
         .replace("##NOTEWURL##", &format!("{}", nurltowurl(name)?));

      Some(res)
   } else {
      None
   }
}

fn get_new_name() -> String {
   let rand = std::collections::hash_map::RandomState::new()
      .build_hasher()
      .finish() % 0xFFFFFF;

   let name = format!("{:06x}", rand);

   if fs::read_dir("notes/").unwrap()
      .any(|x| x.is_ok_and(|x| x.file_name().to_string_lossy() == name)) { get_new_name() } else { name }
}

fn main() {
   let port = if std::env::args().len() < 2 { "8000".to_owned() } else { std::env::args().collect::<Vec<String>>()[1].clone() };

   rouille::start_server(format!("0.0.0.0:{}", port), move |req| {
      router!(req,
         (GET) (/) => {
            rouille::Response::html(EDIT_HTML)
         },

         (GET) (/raw/{note: String}) => {
            let path = if note.len() == 6 { note } else { tryorr!(wurltonurl(note).ok_or(()))};

            let text = tryorr!(fs::read_to_string(format!("notes/{}", path)));

            rouille::Response::text(text)
         },

         (GET) (/{note: String}) => {
            let path = if note.len() == 6 { note } else { tryorr!(wurltonurl(note).ok_or(())) };

            rouille::Response::html(tryorr!(get_note_html(path).ok_or(())))
         },

         (POST) (/) => {
            let path = get_new_name();

            let mut body = String::new();
            try_or_400!(try_or_400!(req.data().ok_or(io::Error::new(io::ErrorKind::Other, "?"))).read_to_string(&mut body));

            try_or_400!(fs::write(format!("notes/{}", path), body));

            rouille::Response::text(path)
         },

         _ => rouille::Response::empty_404()
      )
   });
}
