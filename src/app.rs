use clap::{App, AppSettings, Arg, SubCommand};

/// Build a clap application.
pub fn app() -> App<'static, 'static> {
    let cmd_capture = SubCommand::with_name("capture")
        .setting(AppSettings::UnifiedHelpMessage)
        .arg(Arg::with_name("output"))
        .arg(Arg::with_name("video")
             .long("video")
             .takes_value(true)
             .default_value("/dev/video1"))
        .arg(Arg::with_name("audio")
             .long("audio")
             .takes_value(true)
             .default_value("hw:1,0"))
        .arg(Arg::with_name("duration")
             .takes_value(true)
             .long("duration").short("t")
             .help("A duration to record. This is useful for dry-run tests."))
        .arg(Arg::with_name("tmpdir")
             .takes_value(true)
             .long("tmpdir")
             .default_value("/m/tmp/vcr"))
        .arg(Arg::with_name("resume")
             .takes_value(true)
             .long("resume")
             .help("The timestamp of a capture job to resume."));
    App::new("vcr")
        .author(crate_authors!())
        .version(crate_version!())
        .max_term_width(100)
        .setting(AppSettings::UnifiedHelpMessage)
        .subcommand(cmd_capture)
}
