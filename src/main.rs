use std::{
    fs::{create_dir, write},
    process::exit,
};

use html_template_scraper::scrap_html;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Invalid args");
        exit(1);
    }
    let template_url = args[1].clone();
    let output_dir = args.get(2).unwrap_or(&String::from("output")).clone();
    create_dir(&output_dir)?;
    let scraped_html = scrap_html(template_url).await?;
    write(format!("{}/index.html", output_dir), scraped_html.content)?;
    create_dir(format!("{}/css", output_dir))?;
    create_dir(format!("{}/src", output_dir))?;
    create_dir(format!("{}/img", output_dir))?;
    create_dir(format!("{}/font", output_dir))?;

    for stylesheet in scraped_html.stylesheets {
        write(
            format!("{}/css/{}", output_dir, stylesheet.name),
            stylesheet.content,
        )?;
    }

    for script in scraped_html.scripts {
        write(
            format!("{}/src/{}", output_dir, script.name),
            script.content,
        )?;
    }

    for image in scraped_html.images {
        write(format!("{}/img/{}", output_dir, image.name), image.content)?;
    }

    for font in scraped_html.fonts {
        write(format!("{}/font/{}", output_dir, font.name), font.content)?;
    }

    if let Some(icon) = scraped_html.icon {
        write(format!("{}/{}", output_dir, icon.name), icon.content)?;
    }

    if let Some(shortcut_icon) = scraped_html.shortcut_icon {
        write(
            format!("{}/{}", output_dir, shortcut_icon.name),
            shortcut_icon.content,
        )?;
    }

    Ok(())
}
