#[macro_use] extern crate rocket;
#[macro_use] extern crate lazy_static;

use rocket::fs::FileServer;
use rocket::form::Form;
use rocket::response::Redirect;
use rocket_dyn_templates::{Template, context};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use chrono::NaiveDate;
use std::sync::Mutex;
use pulldown_cmark::{Parser, html::push_html, Options};



#[derive(Debug, Deserialize, Serialize, Clone)]
struct BlogPost {
    title: String,
    date: NaiveDate,
    tags: Vec<String>,
    description: String,
    content: String,
}

lazy_static! {
    static ref POSTS: Mutex<Vec<BlogPost>> = Mutex::new(Vec::new());
}

fn load_posts() {
    let mut posts = POSTS.lock().unwrap();
    let content_dir = Path::new("content/posts");

    if !content_dir.exists() {
        std::fs::create_dir_all(content_dir).expect("Failed to create content directory");
    }

    for entry in fs::read_dir(content_dir).expect("Failed to read content directory") {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).expect("Failed to read file");
                let mut post: BlogPost = serde_json::from_str(&content).expect("Failed to parse JSON");
                // Parse Markdown content
                let mut options = Options::empty();
                options.insert(Options::ENABLE_STRIKETHROUGH);
                let parser = Parser::new_ext(&post.content, options);
                let mut html_output = String::new();
                push_html(&mut html_output, parser);
                post.content = html_output;
                posts.push(post);
            }
        }
    }
    posts.sort_by(|a, b| b.date.cmp(&a.date));
}


#[get("/?<page>&<query>")]
fn index(page: Option<usize>, query: Option<String>) -> Template {
    let posts = POSTS.lock().unwrap();
    let page = page.unwrap_or(1);
    let per_page = 5;
    let total_posts = posts.len();
    let total_pages = (total_posts + per_page - 1) / per_page;

    let filtered_posts: Vec<&BlogPost> = if let Some(ref q) = query {
        posts.iter().filter(|p| {
            p.title.to_lowercase().contains(&q.to_lowercase()) ||
            p.content.to_lowercase().contains(&q.to_lowercase()) ||
            p.tags.iter().any(|t| t.to_lowercase().contains(&q.to_lowercase()))
        }).collect()
    } else {
        posts.iter().collect()
    };

    let start = (page - 1) * per_page;
    let end = std::cmp::min(start + per_page, filtered_posts.len());
    let paginated_posts = &filtered_posts[start..end];

    Template::render("index", context! {
        posts: paginated_posts,
        current_page: page,
        total_pages: total_pages,
        query: query,
    })
}

#[get("/post/<slug>")]
fn post(slug: &str) -> Option<Template> {
    let posts = POSTS.lock().unwrap();
    let post = posts.iter().find(|p| p.title.to_lowercase().replace(" ", "-") == slug)?;
    Some(Template::render("post", context! {
        post: post,
    }))
}

#[get("/tag/<tag>")]
fn tag(tag: &str) -> Template {
    let posts = POSTS.lock().unwrap();
    let tagged_posts: Vec<&BlogPost> = posts.iter().filter(|p| p.tags.contains(&tag.to_string())).collect();
    Template::render("index", context! {
        posts: tagged_posts,
        tag: tag,
    })
}

#[derive(FromForm)]
struct SearchForm {
    query: String,
}

#[post("/search", data = "<search_form>")]
fn search(search_form: Form<SearchForm>) -> Redirect {
    Redirect::to(format!("/?page=1&query={}", search_form.query))
}

#[launch]
fn rocket() -> _ {
    load_posts();
    let static_dir = format!("{}/static", env!("CARGO_MANIFEST_DIR"));
    rocket::build()
        .mount("/", routes![index, post, tag, search])
        .mount("/static", FileServer::from(static_dir))
        .attach(Template::fairing())
}