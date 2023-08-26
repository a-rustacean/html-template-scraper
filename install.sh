cargo build --release
rm -rf $HOME/.local/bin/scrape-html
ln -s $(pwd)/target/release/html-template-scraper $HOME/.local/bin/scrape-html
chmod u+x $HOME/.local/bin/scrape-html
