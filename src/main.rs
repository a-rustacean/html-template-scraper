use std::{
    fs::{create_dir, write},
    process::exit,
};

use html_template_scrapper::scrap_html;

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

    for link in scraped_html.links {
        write(format!("{}/css/{}", output_dir, link.name), link.content)?;
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
    Ok(())
}
