use lazy_static::lazy_static;
use regex::Regex;
use reqwest::IntoUrl;
use scraper::{Html, Selector};
use url::Origin;

lazy_static! {
    static ref FONT_FACE_REGEX: Regex = Regex::new(r"@font-face\s*\{([^}]+)\}").unwrap();
    static ref CSS_URL_REGEX: Regex = Regex::new(r"url\(([^)]+)\)").unwrap();
    static ref CSS_IMPORT_REGEX: Regex =
        Regex::new(r#"(?i)@import\s+url\s*\(\s*(?:"([^"]+)"|'([^']+)'|\(([^)]+)\))\s*\)\s*;"#)
            .unwrap();
}

const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "svg"];
const FONT_EXTENSIONS: [&str; 4] = ["ttf", "eot", "woff", "woff2"];

type AnyError = Box<dyn std::error::Error>;
type AnyResult<T> = Result<T, AnyError>;

#[derive(Debug)]
pub struct ScrapedFile {
    pub name: String,
    pub content: String,
}

#[derive(Debug)]
pub struct ScrapedFileRaw {
    pub name: String,
    pub content: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ScrapedHtml {
    pub content: String,
    pub icon: Option<ScrapedFileRaw>,
    pub shortcut_icon: Option<ScrapedFileRaw>,
    pub stylesheets: Vec<ScrapedFile>,
    pub scripts: Vec<ScrapedFile>,
    pub images: Vec<ScrapedFileRaw>,
    pub fonts: Vec<ScrapedFileRaw>,
    // (Absolute url, file name)
    pub anchors: Vec<(String, String)>,
}

#[allow(dead_code)]
pub struct ScrapedCss {
    pub content: String,
    pub fonts: Vec<ScrapedFileRaw>,
    // (Absolute url, file name)
    pub imports: Vec<(String, String)>,
}

async fn fetch_file_raw<T: IntoUrl>(url: T) -> AnyResult<Vec<u8>> {
    Ok(reqwest::get(url.into_url()?)
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

pub async fn scrap_css<T: IntoUrl>(base: T) -> AnyResult<ScrapedCss> {
    let base = base.into_url().unwrap();
    let mut css = reqwest::get(base.clone()).await?.text().await?;
    let mut fonts = Vec::new();
    let mut imports = Vec::new();

    for font_face_capture in FONT_FACE_REGEX.captures_iter(&css.clone()) {
        let font_face = &font_face_capture[0];
        for src_capture in CSS_URL_REGEX.captures_iter(font_face) {
            let src = src_capture[1].to_string();
            let url = if (src.starts_with('"') && src.ends_with('"'))
                || (src.starts_with('\'') && src.ends_with('\''))
            {
                src[1..src.len() - 1].to_string()
            } else {
                src
            };
            let normal_url = url.split('?').next().unwrap().split('#').next().unwrap();
            let path = match base.join(normal_url) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path_str = path.to_string();
            let file_name = match path_str.split('/').last() {
                Some(v) => v,
                None => continue,
            };
            println!("Font: {}", path_str);
            let file_content = reqwest::get(path).await?.bytes().await?;
            css = css.replace(&url, &format!("../font/{}", file_name));
            fonts.push(ScrapedFileRaw {
                name: file_name.to_string(),
                content: file_content.to_vec(),
            });
        }
    }

    for css_import_capture in CSS_IMPORT_REGEX.captures_iter(&css.clone()) {
        let css_url = css_import_capture
            .get(1)
            .or_else(|| css_import_capture.get(2))
            .or_else(|| css_import_capture.get(3))
            .unwrap()
            .as_str();
        let absolute_url = match base.join(css_url) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let absolute_url_str = absolute_url.to_string();
        let file_name = match absolute_url_str.split('/').last() {
            Some(v) => v,
            None => continue,
        };
        css = css.replace(&css_import_capture[0], file_name);
        imports.push((absolute_url.to_string(), file_name.to_string()));
    }

    Ok(ScrapedCss {
        content: css,
        fonts,
        imports,
    })
}

pub fn extension<T: AsRef<str>>(name: T) -> Option<String> {
    name.as_ref().split('.').last().map(|str| str.into())
}

pub async fn scrap_html<T: IntoUrl>(file_url: T) -> AnyResult<ScrapedHtml> {
    let base = file_url.into_url()?;
    let mut html_file = reqwest::get(base.clone()).await?.text().await?;
    println!("Html: {}", base.clone());
    let html_file_clone = html_file.clone();
    let document = Html::parse_document(&html_file_clone);
    let mut stylesheets = Vec::new();
    let mut scripts = Vec::new();
    let mut images = Vec::new();
    let mut anchors = Vec::new();
    let mut fonts = Vec::new();

    let stylesheet_selector = Selector::parse(r#"link[rel=stylesheet]"#).unwrap();
    let script_selector = Selector::parse(r#"script"#).unwrap();
    let image_selector = Selector::parse(r#"img"#).unwrap();
    let anchor_selector = Selector::parse(r#"a"#).unwrap();
    let icon_selector = Selector::parse(r#"link[rel=icon]"#).unwrap();
    let shortcut_icon_selector = Selector::parse(r#"link[rel="shortcut icon"]"#).unwrap();
    let inline_style_selector = Selector::parse(r#"[style]"#).unwrap();

    let icon = async {
        if let Some(icon) = document.select(&icon_selector).next() {
            if let Some(href) = icon.value().attr("href") {
                if let Ok(href) = base.join(href) {
                    if let Some(file_name) = href.to_string().split('/').last() {
                        if let Ok(content) = fetch_file_raw(href.clone()).await {
                            let href_string = href.to_string();
                            html_file = html_file.replace(&href_string, file_name);
                            println!("Icon: {}", href_string);
                            return Some(ScrapedFileRaw {
                                name: file_name.to_string(),
                                content,
                            });
                        }
                    }
                }
            }
        } else if let Ok(content) = fetch_file_raw(base.join("favicon.ico").unwrap()).await {
            println!("Icon: {}/favicon.ico", base);
            return Some(ScrapedFileRaw {
                name: String::from("favicon.ico"),
                content,
            });
        } else if let Origin::Tuple(protocol, host, port) = base.origin() {
            if let Ok(content) =
                fetch_file_raw(format!("{}://{}:{}/favicon.ico", protocol, host, port)).await
            {
                println!("Icon: {}://{}:{}/favicon.ico", protocol, host, port);
                return Some(ScrapedFileRaw {
                    name: String::from("favicon.ico"),
                    content,
                });
            }
        }
        None
    }
    .await;

    let shortcut_icon = async {
        if let Some(icon) = document.select(&shortcut_icon_selector).next() {
            if let Some(href) = icon.value().attr("href") {
                if let Ok(href) = base.join(href) {
                    if let Some(file_name) = href.to_string().split('/').last() {
                        if let Ok(content) = fetch_file_raw(href.clone()).await {
                            html_file = html_file.replace(&href.to_string(), file_name);
                            println!("Shortcut Icon: {}", href);
                            return Some(ScrapedFileRaw {
                                name: file_name.to_string(),
                                content,
                            });
                        }
                    }
                }
            }
        }
        None
    }
    .await;

    for element in document.select(&stylesheet_selector) {
        match element.value().attr("href") {
            Some(attr) => {
                let absolute_attr = match base.join(attr) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let absolute_attr_str = absolute_attr.to_string();
                let file_name = match absolute_attr_str.split('/').last() {
                    Some(v) => v,
                    None => continue,
                };
                if absolute_attr.host_str() == base.host_str() {
                    println!("Link: {}", absolute_attr_str);
                    let scraped_css = scrap_css(absolute_attr).await;
                    if let Ok(scraped_css) = scraped_css {
                        scraped_css
                            .fonts
                            .into_iter()
                            .for_each(|font| fonts.push(font));
                        html_file = html_file.replace(attr, &format!("css/{}", file_name));
                        stylesheets.push(ScrapedFile {
                            name: file_name.to_string(),
                            content: scraped_css.content,
                        });
                    }
                }
            }
            None => continue,
        }
    }

    for element in document.select(&script_selector) {
        match element.value().attr("src") {
            Some(attr) => {
                let absolute_attr = match base.join(attr) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let absolute_attr_str = absolute_attr.to_string();
                let file_name = match absolute_attr_str.split('/').last() {
                    Some(v) => v,
                    None => continue,
                };
                if absolute_attr.host_str() == base.host_str() {
                    println!("Script: {}", absolute_attr_str);
                    let file_content = match reqwest::get(absolute_attr).await.map(|req| req.text())
                    {
                        Ok(req) => match req.await {
                            Ok(text) => text,
                            Err(_) => continue,
                        },
                        Err(_) => continue,
                    };
                    html_file = html_file.replace(attr, &format!("src/{}", file_name));
                    scripts.push(ScrapedFile {
                        name: file_name.to_string(),
                        content: file_content,
                    });
                }
            }
            None => continue,
        }
    }

    for element in document.select(&image_selector) {
        match element.value().attr("src") {
            Some(attr) => {
                let absolute_attr = match base.join(attr) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let absolute_attr_str = absolute_attr.to_string();
                let file_name = match absolute_attr_str.split('/').last() {
                    Some(v) => v,
                    None => continue,
                };
                if absolute_attr.host_str() == base.host_str() {
                    println!("Image: {}", absolute_attr_str);
                    let file_content =
                        match reqwest::get(absolute_attr).await.map(|req| req.bytes()) {
                            Ok(req) => match req.await {
                                Ok(bytes) => bytes,
                                Err(_) => continue,
                            },
                            Err(_) => continue,
                        };
                    html_file = html_file.replace(attr, &format!("img/{}", file_name));
                    images.push(ScrapedFileRaw {
                        name: file_name.to_string(),
                        content: file_content.to_vec(),
                    });
                }
            }
            None => continue,
        }
    }

    for element in document.select(&anchor_selector) {
        match element.value().attr("href") {
            Some(attr) => {
                if attr.starts_with('#') {
                    continue;
                }
                let absolute_attr = match base.join(attr) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let absolute_attr_str = absolute_attr.to_string();
                let file_name = match absolute_attr_str.split('/').last() {
                    Some(v) => v,
                    None => continue,
                };
                if absolute_attr.host_str() == base.host_str() {
                    println!("Anchor: {}", absolute_attr_str);
                    html_file = html_file.replace(attr, &format!("/{}", file_name));
                    anchors.push((absolute_attr_str.clone(), file_name.to_string()));
                }
            }
            None => continue,
        }
    }

    for element in document.select(&inline_style_selector) {
        match element.value().attr("style") {
            Some(style) => {
                for css_url_capture in CSS_URL_REGEX.captures_iter(style) {
                    let url = css_url_capture[1].to_string();
                    let url = if (url.starts_with('"') && url.ends_with('"'))
                        || (url.starts_with('\'') && url.ends_with('\''))
                    {
                        url[1..url.len() - 1].to_string()
                    } else {
                        url
                    };

                    let absolute_url = match base.join(&url) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    let absolute_url_str = absolute_url.to_string();
                    let file_name = match absolute_url_str.split('/').last() {
                        Some(v) => v,
                        None => continue,
                    };
                    let ext = match extension(file_name) {
                        Some(v) => v,
                        None => continue,
                    };
                    if absolute_url.host_str() == base.host_str() {
                        let req = match reqwest::get(absolute_url).await {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                            let file_content = match req.bytes().await {
                                Ok(v) => v,
                                Err(_) => continue,
                            };
                            println!("Image: {}", absolute_url_str);
                            html_file =
                                html_file.replace(&absolute_url_str, &format!("img/{}", file_name));
                            images.push(ScrapedFileRaw {
                                name: file_name.to_string(),
                                content: file_content.to_vec(),
                            });
                        } else if FONT_EXTENSIONS.contains(&ext.as_str()) {
                            let file_content = match req.bytes().await {
                                Ok(v) => v,
                                Err(_) => continue,
                            };
                            html_file = html_file
                                .replace(&absolute_url_str, &format!("fonts/{}", file_name));
                            println!("Font: {}", absolute_url_str);
                            fonts.push(ScrapedFileRaw {
                                name: file_name.to_string(),
                                content: file_content.to_vec(),
                            });
                        }
                    }
                }
            }
            None => continue,
        }
    }

    Ok(ScrapedHtml {
        content: html_file,
        icon,
        shortcut_icon,
        stylesheets,
        scripts,
        images,
        anchors,
        fonts,
    })
}
