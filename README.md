# resource-sida-omv-rlden
Programs to scrape and prepare Sidas OmVärlden

All programs depend on rust.

## `omvarlden-crawler`

Program to scrape [Sidas OmVärlden](https://www.omvarlden.se).

### Install

Simply run:
```console
cargo build --release
```

The executable is then located in `./target/release/omvarlden-crawler`.

### Usage

Either run the executable from your computer or copy to a linux server of your choice.

Suggestion is to use `nohup` like this:

```console
nohup ./omvarlden-crawler > stdout.txt 2> stderr.json &
```
