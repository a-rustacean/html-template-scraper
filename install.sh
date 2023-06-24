cargo build --release
rm -rf $HOME/.local/bin/scrap-html
ln -s $(pwd)/target/release/html-template-scraper $HOME/.local/bin/scrap-html
chmod u+x $HOME/.local/bin/scrap-html
