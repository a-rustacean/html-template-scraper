use lazy_static::lazy_static;
use regex::Regex;
use reqwest::IntoUrl;
use scraper::{Html, Selector};
use ttf_parser::OutlineBuilder;

lazy_static! {
    static ref FONT_FACE_REGEX: Regex = Regex::new(r"@font-face\s*\{([^}]+)\}").unwrap();
    static ref FONT_FACE_SRC_REGEX: Regex = Regex::new(r"url\(([^)]+)\)").unwrap();
    static ref CSS_IMPORT_REGEX: Regex =
        Regex::new(r#"(?i)@import\s+url\s*\(\s*(?:"([^"]+)"|'([^']+)'|\(([^)]+)\))\s*\)\s*;"#)
            .unwrap();
}

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
    pub links: Vec<ScrapedFile>,
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

struct Builder<'a>(&'a mut String);

impl OutlineBuilder for Builder<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.0, "M {} {} ", x, y).unwrap()
    }

    fn line_to(&mut self, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.0, "L {} {} ", x, y).unwrap()
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.0, "Q {} {} {} {} ", x1, y1, x, y).unwrap()
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.0, "C {} {} {} {} {} {} ", x1, y1, x2, y2, x, y).unwrap()
    }

    fn close(&mut self) {
        self.0.push_str("Z ")
    }
}

pub async fn scrap_css<T: IntoUrl>(base: T) -> Result<ScrapedCss, Box<dyn std::error::Error>> {
    let base = base.into_url().unwrap();
    let mut css = reqwest::get(base.clone()).await?.text().await?;
    let mut fonts = Vec::new();
    let mut imports = Vec::new();

    for font_face_capture in FONT_FACE_REGEX.captures_iter(&css.clone()) {
        let font_face = &font_face_capture[0];
        for src_capture in FONT_FACE_SRC_REGEX.captures_iter(font_face) {
            let src = src_capture[1].to_string();
            let url = if (src.starts_with('"') && src.ends_with('"'))
                || (src.starts_with("'") && src.ends_with("'"))
            {
                src[1..src.len() - 1].to_string()
            } else {
                src
            };
            let normal_url = url.split("?").nth(0).unwrap().split("#").nth(0).unwrap();
            let path = match base.join(&normal_url) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path_str = path.to_string();
            let file_name = match path_str.split("/").last() {
                Some(v) => v,
                None => continue,
            };
            println!("Font: {}", path_str);
            let file_content = reqwest::get(path).await?.bytes().await?;
            let tmp = css.replace(&url, &format!("../font/{}", file_name));
            css = tmp;
            fonts.push(ScrapedFileRaw {
                name: file_name.clone().to_string(),
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
        let file_name = match absolute_url_str.split("/").last() {
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

pub async fn scrap_html<T: IntoUrl>(
    file_url: T,
) -> Result<ScrapedHtml, Box<dyn std::error::Error>> {
    let base = file_url.into_url()?;
    let mut html_file = reqwest::get(base.clone()).await?.text().await?;
    println!("Html: {}", base.clone());
    let html_file_clone = html_file.clone();
    let document = Html::parse_document(&html_file_clone);
    let mut links = Vec::new();
    let mut scripts = Vec::new();
    let mut images = Vec::new();
    let mut anchors = Vec::new();
    let mut fonts = Vec::new();
    println!("Parsed");

    let link_selector = Selector::parse(r#"link[rel=stylesheet]"#).unwrap();
    let script_selector = Selector::parse(r#"script"#).unwrap();
    let image_selector = Selector::parse(r#"img"#).unwrap();
    let anchor_selector = Selector::parse(r#"a"#).unwrap();

    for element in document.select(&link_selector) {
        match element.value().attr("href") {
            Some(attr) => {
                let absolute_attr = match base.join(attr) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let absolute_attr_str = absolute_attr.to_string();
                let file_name = match absolute_attr_str.split("/").last() {
                    Some(v) => v,
                    None => continue,
                };
                if absolute_attr.host_str() == base.host_str() {
                    println!("Link: {}", absolute_attr_str);
                    html_file = html_file.replace(attr, &format!("css/{}", file_name));
                    let scraped_css = scrap_css(absolute_attr).await;
                    if let Ok(scraped_css) = scraped_css {
                        scraped_css
                            .fonts
                            .into_iter()
                            .for_each(|font| fonts.push(font));
                        links.push(ScrapedFile {
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
                let file_name = match absolute_attr_str.split("/").last() {
                    Some(v) => v,
                    None => continue,
                };
                if absolute_attr.host_str() == base.host_str() {
                    println!("Script: {}", absolute_attr_str);
                    html_file = html_file.replace(attr, &format!("src/{}", file_name));
                    let file_content = reqwest::get(absolute_attr).await?.text().await?;
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
                let file_name = match absolute_attr_str.split("/").last() {
                    Some(v) => v,
                    None => continue,
                };
                if absolute_attr.host_str() == base.host_str() {
                    println!("Image: {}", absolute_attr_str);
                    html_file = html_file.replace(attr, &format!("img/{}", file_name));
                    let file_content = reqwest::get(absolute_attr).await?.bytes().await?;
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
                if attr.starts_with("#") {
                    continue;
                }
                let absolute_attr = match base.join(attr) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let absolute_attr_str = absolute_attr.to_string();
                let file_name = match absolute_attr_str.split("/").last() {
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

    Ok(ScrapedHtml {
        content: html_file,
        links,
        scripts,
        images,
        anchors,
        fonts,
    })
}
