#![feature(test)]

extern crate clap;
extern crate test;

use test::Bencher;

use std::io::Cursor;

use clap::App;
use clap::{Arg, ArgSettings};

fn build_help(app: &mut App) -> String {
    let mut buf = Cursor::new(Vec::with_capacity(50));
    app.write_help(&mut buf).unwrap();
    let content = buf.into_inner();
    String::from_utf8(content).unwrap()
}

fn app_example1<'b, 'c>() -> App<'b, 'c> {
    App::new("MyApp")
        .version("1.0")
        .author("Kevin K. <kbknapp@gmail.com>")
        .about("Does awesome things")
        .arg("-c, --config=[FILE] 'Sets a custom config file'")
        .arg("<output> 'Sets an optional output file'")
        .arg("-d... 'Turn debugging information on'")
        .subcommand(
            App::new("test")
                .about("does testing things")
                .arg("-l, --list 'lists test values'"),
        )
}

fn app_example2<'b, 'c>() -> App<'b, 'c> {
    App::new("MyApp")
        .version("1.0")
        .author("Kevin K. <kbknapp@gmail.com>")
        .about("Does awesome things")
}

fn app_example3<'b, 'c>() -> App<'b, 'c> {
    App::new("MyApp")
        .arg(
            Arg::new("debug")
                .help("turn on debugging information")
                .short('d'),
        )
        .args(&[
            Arg::new("config")
                .help("sets the config file to use")
                .setting(ArgSettings::TakesValue)
                .short('c')
                .long("config"),
            Arg::new("input")
                .help("the input file to use")
                .index(1)
                .setting(ArgSettings::Required),
        ])
        .arg("--license 'display the license file'")
        .arg("[output] 'Supply an output file to use'")
        .arg("-i, --int=[IFACE] 'Set an interface to use'")
}

fn app_example4<'b, 'c>() -> App<'b, 'c> {
    App::new("MyApp")
        .about("Parses an input file to do awesome things")
        .version("1.0")
        .author("Kevin K. <kbknapp@gmail.com>")
        .arg(
            Arg::new("debug")
                .help("turn on debugging information")
                .short('d')
                .long("debug"),
        )
        .arg(
            Arg::new("config")
                .help("sets the config file to use")
                .short('c')
                .long("config"),
        )
        .arg(
            Arg::new("input")
                .help("the input file to use")
                .index(1)
                .setting(ArgSettings::Required),
        )
}

fn app_example5<'b, 'c>() -> App<'b, 'c> {
    App::new("MyApp").arg(
        Arg::new("awesome")
            .help("turns up the awesome")
            .short('a')
            .long("awesome")
            .setting(ArgSettings::MultipleOccurrences)
            .requires("config")
            .conflicts_with("output"),
    )
}

fn app_example6<'b, 'c>() -> App<'b, 'c> {
    App::new("MyApp")
        .arg(
            Arg::new("input")
                .help("the input file to use")
                .index(1)
                .requires("config")
                .conflicts_with("output")
                .setting(ArgSettings::Required),
        )
        .arg(
            Arg::new("config")
                .help("the config file to use")
                .index(2),
        )
}

fn app_example7<'b, 'c>() -> App<'b, 'c> {
    App::new("MyApp")
        .arg(Arg::new("config"))
        .arg(Arg::new("output"))
        .arg(
            Arg::new("input")
                .help("the input file to use")
                .settings(&[
                    ArgSettings::MultipleValues,
                    ArgSettings::MultipleOccurrences,
                    ArgSettings::Required,
                ])
                .short('i')
                .long("input")
                .requires("config")
                .conflicts_with("output"),
        )
}

fn app_example8<'b, 'c>() -> App<'b, 'c> {
    App::new("MyApp")
        .arg(Arg::new("config"))
        .arg(Arg::new("output"))
        .arg(
            Arg::new("input")
                .help("the input file to use")
                .settings(&[
                    ArgSettings::MultipleValues,
                    ArgSettings::MultipleOccurrences,
                    ArgSettings::Required,
                ])
                .short('i')
                .long("input")
                .requires("config")
                .conflicts_with("output"),
        )
}

fn app_example10<'b, 'c>() -> App<'b, 'c> {
    App::new("myapp").about("does awesome things").arg(
        Arg::new("CONFIG")
            .help("The config file to use (default is \"config.json\")")
            .short('c')
            .setting(ArgSettings::TakesValue),
    )
}

#[bench]
fn example1(b: &mut Bencher) {
    let mut app = app_example1();
    b.iter(|| build_help(&mut app));
}

#[bench]
fn example2(b: &mut Bencher) {
    let mut app = app_example2();
    b.iter(|| build_help(&mut app));
}

#[bench]
fn example3(b: &mut Bencher) {
    let mut app = app_example3();
    b.iter(|| build_help(&mut app));
}

#[bench]
fn example4(b: &mut Bencher) {
    let mut app = app_example4();
    b.iter(|| build_help(&mut app));
}

#[bench]
fn example5(b: &mut Bencher) {
    let mut app = app_example5();
    b.iter(|| build_help(&mut app));
}

#[bench]
fn example6(b: &mut Bencher) {
    let mut app = app_example6();
    b.iter(|| build_help(&mut app));
}

#[bench]
fn example7(b: &mut Bencher) {
    let mut app = app_example7();
    b.iter(|| build_help(&mut app));
}

#[bench]
fn example8(b: &mut Bencher) {
    let mut app = app_example8();
    b.iter(|| build_help(&mut app));
}

#[bench]
fn example10(b: &mut Bencher) {
    let mut app = app_example10();
    b.iter(|| build_help(&mut app));
}

#[bench]
fn example4_template(b: &mut Bencher) {
    let mut app = app_example4().help_template("{bin} {version}\n{author}\n{about}\n\nUSAGE:\n    {usage}\n\nFLAGS:\n{flags}\n\nARGS:\n{args}\n");
    b.iter(|| build_help(&mut app));
}
