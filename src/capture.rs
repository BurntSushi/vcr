use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{UNIX_EPOCH, SystemTime};

use clap;
use regex::Regex;

use blue::blue_frame_path;
use Result;

pub fn run(args: &clap::ArgMatches) -> Result<()> {
    let config = Config::from_matches(args)?;
    eprintln!("[VCR CONFIG] {:?}", config);
    let initial = InitialRecording::record(&config)?;
    eprintln!("[VCR INITIAL RECORDING] {:?}", initial);
    let intervals = BlueDetect::detect(&config, &initial.0)?;
    eprintln!("[VCR BLUE INTERVALS] {:?}", intervals);
    let trimmed = initial.trim(&config, &intervals)?;
    eprintln!("[VCR TRIMMED RECORDING] {:?}", trimmed);

    eprintln!("[VCR] copying {:?} to {:?}", trimmed.0, config.output);
    fs::copy(&trimmed.0, &config.output)?;
    Ok(())
}

#[derive(Debug)]
struct Config {
    tmpdir: PathBuf,
    output: PathBuf,
    output_name: String,
    video: OsString,
    audio: OsString,
    duration: Option<OsString>,
}

impl Config {
    fn from_matches(args: &clap::ArgMatches) -> Result<Config> {
        let parent_tmpdir = PathBuf::from(
            args.value_of_os("tmpdir").unwrap().to_os_string());
        let output = PathBuf::from(
            args.value_of_os("output").unwrap().to_os_string());
        let video = args.value_of_os("video").unwrap().to_os_string();
        let audio = args.value_of_os("audio").unwrap().to_os_string();
        let duration = args.value_of_os("duration").map(|x| x.to_os_string());

        if output.extension().map_or(false, |x| x != "mp4") {
            bail!("output path must have .mp4 extension");
        }
        let output_name = match Path::new(&output).file_name() {
            None => bail!("output path has no base name: {:?}", output),
            Some(output_name) => output_name.to_string_lossy().into_owned(),
        };
        let tmpdir = match args.value_of_os("resume") {
            None => {
                let stamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)?
                    .as_secs();
                let name = PathBuf::from(format!("{}-{}", output_name, stamp));
                let tmpdir = parent_tmpdir.join(name);
                fs::create_dir(&tmpdir)?;
                tmpdir
            }
            Some(resume) => {
                let stamp = resume.to_string_lossy();
                let name = PathBuf::from(format!("{}-{}", output_name, stamp));
                let tmpdir = parent_tmpdir.join(name);
                if !tmpdir.exists() {
                    bail!("no such job exists: {}", tmpdir.display());
                }
                tmpdir
            }
        };

        Ok(Config {
            tmpdir: tmpdir,
            output: output,
            output_name: output_name,
            video: video,
            audio: audio,
            duration: duration,
        })
    }
}

/// InitialRecording represents a file path to the initial recording from the
/// VCR. This recording is very likely way too long, since normal operation
/// dictates that we let the capture run much longer than the tape. Subsequent
/// steps detect the end of the tape and trim the video.
#[derive(Debug)]
struct InitialRecording(PathBuf);

impl InitialRecording {
    fn record(conf: &Config) -> Result<InitialRecording> {
        let output = conf.tmpdir.join("initial-recording.mkv");
        if output.exists() {
            eprintln!(
                "[VCR] {} already exists, skipping recording",
                output.display());
            return Ok(InitialRecording(output));
        }

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-f").arg("v4l2")
            .arg("-standard").arg("NTSC")
            .arg("-thread_queue_size").arg("1024")
            .arg("-framerate").arg("29.97")
            .arg("-i").arg(&conf.video)
            .arg("-f").arg("alsa")
            .arg("-thread_queue_size").arg("1024")
            .arg("-i").arg(&conf.audio)
            .arg("-framerate").arg("29.97")
            .arg("-vf").arg("yadif")
            .arg("-ac").arg("2")
            .arg("-crf").arg("24");
        if let Some(ref duration) = conf.duration {
            cmd.arg("-t").arg(duration);
        }
        cmd.arg(&output);
        run_command(&mut cmd)?;
        Ok(InitialRecording(output))
    }

    fn trim(
        &self,
        conf: &Config,
        blues: &BlueDetect,
    ) -> Result<TrimmedRecording> {
        let output_path = conf.tmpdir.join("trimmed-recording.mp4");
        if output_path.exists() {
            eprintln!(
                "[VCR] {} already exists, skipping trimmed recording",
                output_path.display());
            return Ok(TrimmedRecording(output_path));
        }

        if blues.0.len() == 0 {
            // woohoo?
            // We want mp4 though, so do a cheap copy.
            eprintln!("[VCR] no blue frames detected, doing cheap conversion");
            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-i").arg(&self.0)
                .arg("-codec").arg("copy")
                .arg(&output_path);
            run_command(&mut cmd)?;
            Ok(TrimmedRecording(output_path))
        } else if blues.0.len() > 1 {
            // Something went wrong. There's probably heuristics we can use
            // here, but let's just do the final transcode. That way, we can
            // just manually slice it very quickly using `-codec copy`.
            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-i").arg(&self.0).arg(&output_path);
            run_command(&mut cmd)?;
            Ok(TrimmedRecording(output_path))
        } else {
            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-t").arg((blues.0[0].start + 5.0).to_string())
                .arg("-i").arg(&self.0)
                .arg(&output_path);
            run_command(&mut cmd)?;
            Ok(TrimmedRecording(output_path))
        }
    }
}

/// TrimmedRecording represents a file path to the trimmed recording. A trimmed
/// recording has its final blue frame suffix removed.
#[derive(Debug)]
struct TrimmedRecording(PathBuf);

/// The results of running ffmpeg's `blackdetect` filter over the original
/// recording blended with a blue frame. The blending results in black frames
/// in place of blue frames, which permits ffmpeg's blackdetect filter to work.
#[derive(Debug)]
struct BlueDetect(Vec<BlueInterval>);

impl BlueDetect {
    fn detect(conf: &Config, video: &Path) -> Result<BlueDetect> {
        let blue_path = blue_frame_path()?;
        let output_path = conf.tmpdir.join("blue-frames.out");

        if output_path.exists() {
            eprintln!(
                "[VCR] {} already exists, skipping blue frame detection",
                output_path.display());
        } else {
            let output = File::create(&output_path)?;
            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-i").arg(video)
                .arg("-loop").arg("1")
                .arg("-i").arg(&blue_path)
                .arg("-filter_complex")
                .arg("[0:v][1:v]blend=difference:shortest=1,blackdetect=d=10")
                .arg("-f").arg("null")
                .arg("-");
            cmd.stdout(output.try_clone()?).stderr(output.try_clone()?);
            run_command(&mut cmd)?;
        }

        let mut intervals = vec![];
        let output = io::BufReader::new(File::open(&output_path)?);
        for line in output.lines() {
            if let Some(interval) = BlueInterval::from_line(&line?)? {
                intervals.push(interval);
            }
        }
        Ok(BlueDetect(intervals))
    }
}

/// The interval (in seconds) of blue frames.
#[derive(Debug)]
struct BlueInterval {
    start: f64,
    end: f64,
    duration: f64,
}

impl BlueInterval  {
    fn from_line(line: &str) -> Result<Option<BlueInterval>> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(?x)
                black_start:(?P<start>[0-9.]+)
                \s+
                black_end:(?P<end>[0-9.]+)
                \s+
                black_duration:(?P<duration>[0-9.]+)
            ").unwrap();
        }
        let caps = match RE.captures(line) {
            None => return Ok(None),
            Some(caps) => caps,
        };
        Ok(Some(BlueInterval {
            start: caps["start"].parse()?,
            end: caps["end"].parse()?,
            duration: caps["duration"].parse()?,
        }))
    }
}

/// Runs a command in a standardized fashion:
///
/// 1. Prints the command to stderr.
/// 2. If the command exits unsuccessfully, return an error.
/// 3. One exception: if the command was sent SIGTERM, then assume everything
///    is probably still OK.
/// 4. On success, return nothing.
fn run_command(cmd: &mut Command) -> Result<()> {
    eprintln!("[VCR COMMAND] {:?}", cmd);
    let status = cmd.status()?;
    if !status.success() {
        if status.code() == Some(255) {
            // This appears to be ffmpeg's behavior when it gets SIGTERM'd.
            // ffmpeg will gracefully shutdown, so we should mush on.
            return Ok(());
        }
        bail!("command exited with code {:?}", status.code());
    }
    Ok(())
}
