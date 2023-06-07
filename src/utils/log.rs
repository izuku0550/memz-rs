#[cfg(feature = "DEBUG_MODE")]
use chrono::Local;
#[cfg(feature = "DEBUG_MODE")]
use log::{error, info, LevelFilter};
#[cfg(feature = "DEBUG_MODE")]
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Logger, Root},
    encode::pattern::PatternEncoder,
    Config,
};

pub enum LogType {
    ERROR,
    INFO,
}

pub enum LogLocation {
    MSG,
    LOG,
    ALL,
}

#[cfg(feature = "DEBUG_MODE")]
pub fn new_log() {
    let now = Local::now();
    let time = now.format("%Y-%m-%d_%H-%M-%S.log").to_string();

    let info_log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(format!("log/INFO_{}", time))
        .unwrap();

    let error_log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(format!("log/ERROR_{}", time))
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("info_log_file", Box::new(info_log_file)))
        .appender(Appender::builder().build("error_log_file", Box::new(error_log_file)))
        .logger(
            Logger::builder()
                .additive(false)
                .appender("info_log_file")
                .build("info_log", LevelFilter::Info),
        )
        .logger(
            Logger::builder()
                .additive(false)
                .appender("error_log_file")
                .build("err_log", LevelFilter::Error),
        )
        .build(Root::builder().build(LevelFilter::Info))
        .unwrap();

    log4rs::init_config(config).unwrap();
}

#[cfg(feature = "DEBUG_MODE")]
pub fn write_log(log: LogType, location: LogLocation, text: &str) {
    match log {
        LogType::INFO => match location {
            LogLocation::MSG => {
                println!("INFO - {}", text);
            }
            LogLocation::LOG => {
                info!(target: "info_log", "{}", text);
            }
            LogLocation::ALL => {
                println!("INFO - {}", text);
                info!(target: "info_log", "{}", text);
            }
        },
        LogType::ERROR => match location {
            LogLocation::MSG => {
                eprintln!("ERROR - {}", text);
            }
            LogLocation::LOG => {
                error!(target: "err_log", "{}", text);
            }
            LogLocation::ALL => {
                eprintln!("ERROR - {}", text);
                error!(target: "err_log", "{}", text);
            }
        },
    }
}
