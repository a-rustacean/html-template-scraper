# HTML Template Scraper

[![Lint and Check formating](https://github.com/a-rustacean/html-template-scraper/actions/workflows/lint.yml/badge.svg)](https://github.com/a-rustacean/html-template-scraper/actions/workflows/lint.yml)

This tool provides a convenient solution by allowing you to access and download a wide range of HTML website templates,
including the high-quality and costly ones offered by companies like [Bootstrap](https://themes.getbootstrap.com/). These templates are known for their exceptional
design and functionality. However, acquiring them usually involves a significant expense. With the assistance of web scraping
techniques, this tool enables you to bypass the financial barrier and obtain these sought-after templates without any charge.
It provides an effective way to acquire a wide variety of expertly produced HTML templates by automating the process of gathering
template data from the web, empowering people and organisations to improve their web presence without incurring significant fees.

> **Note:** Unauthorized scraping of paid content may violate terms of service or copyright laws.


## Key Features

- Scrapes paid HTML templates from various sources
- Provides free access to high-quality templates
- Easy installation and usage
- Customizable scraping behavior

## Installation

Pre-built binaries for Font Icons Scraper are not currently available, you can build it from source, follow these steps:

1. Make sure you have rust installed on your machine.
2. Clone this repository to your local machine.
3. Navigate to the project directory.
4. Run the provided installation script:

```shell
./install.sh
```

This will set up the necessary dependencies and configurations for the scraper.

## Usage

Once the installation is complete, you can run the html scraper with the following command.

```shell
scrape-html <TEMPLATE URL> <OUTPUT FOLDER> <DEPTH> # depth of the recursive css scraping, default is 5
```

The scraper will start fetching HTML templates from various sources and make them available for free download. You can customize the scraping behavior by modifying the code according to your requirements.

## Uninstall

To uninstall the web scraper, follow these steps:

1. Navigate to the project directory.
2. Run the provided uninstallation script:

```shell
./uninstall.sh
```

## Contributing

Contributions to this project are welcome! If you find any issues, have suggestions for improvements, or would like to add missing features,
please feel free to submit a pull request. Although there is no strict PR template, please provide a clear description and follow best
practices for code contributions.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
