// Third Party
#[cfg(feature = "yaml")]
use yaml_rust::Yaml;

// Internal
use ArgMatches;

/// The abstract representation of a command line subcommand.
///
/// This struct describes all the valid options of the subcommand for the program. Subcommands are
/// essentially "sub-[`App`]s" and contain all the same possibilities (such as their own
/// [arguments], subcommands, and settings).
///
/// # Examples
///
/// ```rust
/// # use clap::{App, Arg, };
/// App::new("myprog")
///     .subcommand(
///         App::new("config")
///             .about("Used for configuration")
///             .arg(Arg::new("config_file")
///                 .help("The configuration file to use")
///                 .index(1)))
/// # ;
/// ```
/// [`App`]: ./struct.App.html
/// [arguments]: ./struct.Arg.html
#[derive(Debug, Clone)]
pub struct SubCommand {
    #[doc(hidden)]
    pub id: u64,
    #[doc(hidden)]
    pub matches: ArgMatches,
}
